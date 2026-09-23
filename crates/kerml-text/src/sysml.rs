//! SysML textual construction over an accepted immutable KerML dependency.
//!
//! Current-graph construction is separate from SysML semantic producer closure.
//! Unsupported productions and unresolved references are explicit errors/gaps.
use crate::{
    FrontendDiagnostic, FrontendDiagnosticDomain, ReferenceAssertion,
    library::{
        self, CanonicalKermlStandardLibraries, LibraryDraft, LibraryLoadError, LibrarySourceMap,
        ReferenceRefinementStrategy,
        construction::{self, SourceInput},
    },
};
use agq_kerml::{classes as c, properties as p};
use agq_kerml_semantics::{Completeness, KerMlQueries, Resolution};
use agq_kerml_syntax::{ReferenceKind, Visibility, production};
use agq_kernel::{
    ElementId, ModelView, Snapshot,
    provenance::{DeclaredOrigin, FactKey},
    value::Value,
};
use agq_standard_libraries::{LibraryLanguage, VerifiedLibrarySet};
use std::{collections::BTreeSet, sync::Arc};
mod publication;
pub use publication::*;
mod source;
pub(crate) use source::{AcceptedSourceDependency, lower_accepted_source};
#[cfg(test)]
#[path = "sysml_index_shape_tests.rs"]
mod index_shape_tests;
#[cfg(test)]
#[path = "sysml_tests.rs"]
mod tests;

/// Exact-source construction status, never a Systems Library publication claim.
#[derive(Clone, Debug)]
pub struct SystemsDocumentStatus {
    pub path: String,
    pub document: agq_kernel::DocumentId,
    pub source_sha256: String,
    pub profile: production::SysmlSyntaxProfile,
    pub parsed: bool,
    pub byte_exact: bool,
    pub recovery_count: usize,
    pub production_count: usize,
    pub construction_gap: Option<String>,
}

/// Unpublished canonical records for the currently supported Systems documents.
/// Missing documents mark all candidate namespace lookups incomplete.
#[derive(Debug)]
pub struct SystemsLibraryCandidate {
    draft: LibraryDraft,
    documents: Vec<SystemsDocumentStatus>,
    publication: Arc<CanonicalKermlStandardLibraries>,
    source_content_set: String,
    syntax_profile: production::SysmlSyntaxProfile,
    production: Option<SystemsConstructionProduction>,
    dependency_contract: agq_sysml_semantics::SysmlDependencyContract,
}

/// Unpublished combined worklist result, separate from canonical acceptance.
#[derive(Debug)]
pub struct SystemsConstructionProduction {
    /// False identifies a reference bootstrap. The strict publication gate
    /// always runs all predicates after declared endpoint reconstruction.
    pub final_predicates: bool,
    pub completeness: Completeness,
    pub converged: bool,
    pub stages: Vec<agq_kerml_semantics::PublicationStage>,
    pub counters: agq_kerml_semantics::PublicationCounters,
    /// Revalidated evaluations from the previous declared reconstruction.
    pub retained_closure_evaluations: usize,
    /// Previous evaluations reopened because their semantic inputs changed.
    pub reopened_closure_evaluations: usize,
    /// Checked graph rebindings across all construction and predicate passes.
    pub closure_rebindings: usize,
    /// Evaluations retained across those rebindings, including earlier passes.
    pub total_retained_closure_evaluations: usize,
    /// Evaluations reopened across those rebindings, including earlier passes.
    pub total_reopened_closure_evaluations: usize,
}
impl SystemsConstructionProduction {
    /// Known pinned authority conflicts on the final frontier. Transient
    /// diagnostics from earlier frontiers do not establish a current conflict.
    pub fn authority_conflicts(&self) -> Vec<SystemsAuthorityConflict> {
        publication::final_authority_conflicts(&self.stages)
    }
}
impl SystemsLibraryCandidate {
    pub fn draft(&self) -> &LibraryDraft {
        &self.draft
    }
    pub fn documents(&self) -> &[SystemsDocumentStatus] {
        &self.documents
    }
    pub fn source_content_set(&self) -> &str {
        &self.source_content_set
    }
    pub fn syntax_profile(&self) -> production::SysmlSyntaxProfile {
        self.syntax_profile
    }
    pub fn accepted_kerml(&self) -> &Arc<CanonicalKermlStandardLibraries> {
        &self.publication
    }
    pub fn production(&self) -> Option<&SystemsConstructionProduction> {
        self.production.as_ref()
    }
    pub fn dependency_contract(&self) -> &agq_sysml_semantics::SysmlDependencyContract {
        &self.dependency_contract
    }
    /// Source/graph construction completeness only; SysML producers are separate.
    pub fn construction_complete(&self) -> bool {
        self.documents.iter().all(|document| {
            document.parsed && document.byte_exact && document.construction_gap.is_none()
        }) && self.draft.candidate().obligations().is_empty()
    }
    pub fn queries(&self) -> Result<KerMlQueries<'_>, LibraryLoadError> {
        // Exact missing reference fields already participate as construction
        // obligations. Only missing source populations make every root pending.
        let pending = if self
            .documents
            .iter()
            .all(|document| document.parsed && document.construction_gap.is_none())
        {
            BTreeSet::new()
        } else {
            self.draft.roots().iter().copied().collect()
        };
        systems_candidate_queries(
            &self.draft,
            &self.publication,
            pending,
            &self.dependency_contract,
        )
    }
}

/// Parse the exact pinned 21 Systems documents using the strict shared frontend,
/// retaining individual grammar/construction gaps. No recovery node is lowered.
pub fn prepare_systems_library(
    sources: &VerifiedLibrarySet,
    publication: Arc<CanonicalKermlStandardLibraries>,
) -> Result<SystemsLibraryCandidate, LibraryLoadError> {
    prepare_systems_library_with_profile(
        sources,
        publication,
        production::SysmlSyntaxProfile::Published,
    )
}

/// Interpret the exact pinned corpus under explicit textual authority. This
/// closes declared references against the sealed KerML dependency; it does not
/// assert completion of semantic producers or accept a Systems publication.
pub fn prepare_systems_library_with_profile(
    sources: &VerifiedLibrarySet,
    publication: Arc<CanonicalKermlStandardLibraries>,
    profile: production::SysmlSyntaxProfile,
) -> Result<SystemsLibraryCandidate, LibraryLoadError> {
    prepare_systems_library_with_progress(sources, publication, profile, |_| {})
}

/// As `prepare_systems_library_with_profile`, with refinement observations for
/// external resource watchdogs. Observations cannot affect canonical selection.
pub fn prepare_systems_library_with_progress(
    sources: &VerifiedLibrarySet,
    publication: Arc<CanonicalKermlStandardLibraries>,
    profile: production::SysmlSyntaxProfile,
    progress: impl FnMut(&library::ReferenceRefinementRound),
) -> Result<SystemsLibraryCandidate, LibraryLoadError> {
    prepare_systems_library_with_semantic_progress(
        sources,
        publication,
        profile,
        progress,
        |_, _, _, _| {},
        |_| {},
    )
}

/// Observe both reference refinement and combined producer frontiers. The
/// accepted KerML dependency is never part of the scheduled population.
pub fn prepare_systems_library_with_semantic_progress(
    sources: &VerifiedLibrarySet,
    publication: Arc<CanonicalKermlStandardLibraries>,
    profile: production::SysmlSyntaxProfile,
    progress: impl FnMut(&library::ReferenceRefinementRound),
    batch_progress: impl FnMut(usize, usize, usize, usize),
    producer_progress: impl FnMut(&agq_kerml_semantics::PublicationStage),
) -> Result<SystemsLibraryCandidate, LibraryLoadError> {
    prepare_systems_library_scope(
        sources,
        publication,
        profile,
        None,
        progress,
        batch_progress,
        producer_progress,
    )
}

/// Construct a bounded set of exact Systems source paths for producer and
/// reference preflights. Callers must include the slice's Systems dependencies.
/// A slice cannot pass the canonical publication's exact 21-document gate.
pub fn prepare_systems_library_slice_with_semantic_progress(
    sources: &VerifiedLibrarySet,
    publication: Arc<CanonicalKermlStandardLibraries>,
    profile: production::SysmlSyntaxProfile,
    paths: &BTreeSet<String>,
    progress: impl FnMut(&library::ReferenceRefinementRound),
    batch_progress: impl FnMut(usize, usize, usize, usize),
    producer_progress: impl FnMut(&agq_kerml_semantics::PublicationStage),
) -> Result<SystemsLibraryCandidate, LibraryLoadError> {
    if paths.is_empty()
        || paths.iter().any(|path| {
            !sources
                .documents()
                .any(|source| source.language() == LibraryLanguage::SysMl && source.path() == path)
        })
    {
        return Err(LibraryLoadError::Interpretation(
            "Systems slice requires a nonempty set of exact pinned SysML document paths".into(),
        ));
    }
    prepare_systems_library_scope(
        sources,
        publication,
        profile,
        Some(paths),
        progress,
        batch_progress,
        producer_progress,
    )
}

fn prepare_systems_library_scope(
    sources: &VerifiedLibrarySet,
    publication: Arc<CanonicalKermlStandardLibraries>,
    profile: production::SysmlSyntaxProfile,
    paths: Option<&BTreeSet<String>>,
    mut progress: impl FnMut(&library::ReferenceRefinementRound),
    mut batch_progress: impl FnMut(usize, usize, usize, usize),
    mut producer_progress: impl FnMut(&agq_kerml_semantics::PublicationStage),
) -> Result<SystemsLibraryCandidate, LibraryLoadError> {
    if sources.content_set_id() != publication.source_content_set() {
        return Err(LibraryLoadError::Interpretation(
            "Systems sources and accepted KerML source set differ".into(),
        ));
    }
    let mut parsed = Vec::new();
    let semantic_profile = match profile {
        production::SysmlSyntaxProfile::Published => {
            agq_sysml_semantics::SysmlBaselineProfile::Published
        }
        production::SysmlSyntaxProfile::OperationalV1 => {
            agq_sysml_semantics::SysmlBaselineProfile::OperationalV1
        }
        production::SysmlSyntaxProfile::OperationalV2 => {
            agq_sysml_semantics::SysmlBaselineProfile::OperationalV2
        }
    };
    let candidate_bindings = agq_sysml_semantics::StandardSysmlBindings::unbound(
        agq_sysml_semantics::SystemsLibraryIdentity::pinned(
            agq_sysml_semantics::SystemsLibraryIdentity::SOURCE_CONTENT_SET,
        ),
    );
    let dependency_contract = agq_sysml_semantics::SysmlDependencyContract::checked_in_for_profile(
        &candidate_bindings,
        semantic_profile,
    )
    .map_err(|error| LibraryLoadError::Interpretation(error.to_string()))?;
    if dependency_contract.kerml_publication_digest != publication.semantic_digest()
        || dependency_contract.kerml_profile != publication.profile().id()
        || dependency_contract.kerml_source_content_set != sources.content_set_id()
    {
        return Err(LibraryLoadError::Interpretation(
            "Systems interpretation requires the exact accepted KerML Operational v9 publication"
                .into(),
        ));
    }
    let mut documents = Vec::new();
    let base = base(&publication)?;
    // Syntax support is independent of reference endpoints. Probe each document
    // against the same small descriptor registry, without retaining 21 copies
    // of accepted dependency navigation indexes.
    let probe = Snapshot::new(Arc::new(base.model().registry().clone()));
    for source in sources
        .documents()
        .filter(|source| source.language() == LibraryLanguage::SysMl)
        .filter(|source| paths.is_none_or(|paths| paths.contains(source.path())))
    {
        let syntax = production::parse_sysml_with_profile(
            profile,
            source.document(),
            source.revision(),
            source.source(),
            production::Limits::default(),
        )?;
        let byte_exact = syntax
            .tokens()
            .iter()
            .map(|token| syntax.token_text(token))
            .collect::<String>()
            == source.source();
        let construction_gap = if syntax.is_complete() {
            construction::check_supported_sysml(&syntax)
                .and_then(|()| {
                    // Probe the same actions that aggregate construction uses,
                    // including modifiers and mandatory syntactic synthesis.
                    // Missing reference endpoints remain ordinary obligations.
                    construction::construct_on(
                        &[SourceInput {
                            syntax: &syntax,
                            library: Some(source),
                            sysml: true,
                        }],
                        &Default::default(),
                        publication.profile(),
                        probe.clone(),
                        None,
                    )
                    .map(|_| ())
                })
                .err()
                .map(|error| error.to_string())
        } else {
            Some(format!(
                "Selected SysML grammar requires recovery at {:?}",
                syntax.recovery()
            ))
        };
        let supported = construction_gap.is_none();
        documents.push(SystemsDocumentStatus {
            path: source.path().into(),
            document: source.document(),
            source_sha256: source.sha256().into(),
            profile,
            parsed: syntax.is_complete(),
            byte_exact,
            recovery_count: syntax.recovery().len(),
            production_count: syntax.nodes().count(),
            construction_gap,
        });
        if supported {
            parsed.push((source, syntax));
        }
    }
    let inputs: Vec<_> = parsed
        .iter()
        .map(|(source, syntax)| SourceInput {
            syntax,
            library: Some(source),
            sysml: true,
        })
        .collect();
    let all_supported = documents
        .iter()
        .all(|document| document.construction_gap.is_none());
    let mut draft = library::refinement::refine(
        |resolved| {
            construction::construct_on(&inputs, resolved, publication.profile(), base.clone(), None)
        },
        |draft| {
            let pending = if all_supported {
                BTreeSet::new()
            } else {
                draft.roots().iter().copied().collect()
            };
            Ok(
                systems_candidate_queries(draft, &publication, pending, &dependency_contract)?
                    .status_queries(),
            )
        },
        ReferenceRefinementStrategy::DependencyDriven,
        &mut progress,
    )?;
    let mut production = None;
    let mut closure_rebindings = 0;
    let mut total_retained_closure_evaluations = 0;
    let mut total_reopened_closure_evaluations = 0;
    if all_supported
        && matches!(
            profile,
            production::SysmlSyntaxProfile::OperationalV1
                | production::SysmlSyntaxProfile::OperationalV2
        )
    {
        // Positive inheritance normally supplies source endpoints without
        // evaluating scalar predicates that would be discarded by the next
        // declared reconstruction. Retain a full construction fallback for
        // references that need those later consequences; neither pass accepts
        // a publication. The strict facade independently closes the final input.
        for final_predicates in [false, true] {
            let registry = systems_producer_registry(publication.profile());
            // A compact checkpoint survives releasing the previous graph. It
            // carries semantic read fingerprints, never stale derived records.
            let mut closure_checkpoint: Option<agq_kerml_semantics::ProducerClosureCheckpoint> =
                None;
            let endpoints = draft
                .references()
                .iter()
                .filter_map(|reference| {
                    draft
                        .candidate()
                        .model()
                        .navigation_slot(reference.relationship, reference.property)
                        .and_then(|slot| {
                            slot.value().values().find_map(|value| {
                                if let Value::Reference(id) = value {
                                    Some(((reference.relationship, reference.property), *id))
                                } else {
                                    None
                                }
                            })
                        })
                })
                .collect();
            // Reconstruct only declared records. Derived endpoints and ownership
            // remain in a distinct unpublished overlay and are never copied back.
            drop(draft);
            draft = library::refinement::refine_from(
                endpoints,
                |resolved| {
                    let mut current = construction::construct_on(
                        &inputs,
                        resolved,
                        publication.profile(),
                        base.clone(),
                        None,
                    )?;
                    let roots: Vec<_> = current
                        .roots()
                        .iter()
                        .chain(publication.roots())
                        .copied()
                        .collect();
                    let extension = if final_predicates {
                        agq_sysml_semantics::SysmlProducerExtension::new(
                            semantic_profile,
                            candidate_bindings.clone(),
                            roots,
                        )
                    } else {
                        agq_sysml_semantics::SysmlProducerExtension::for_reference_refinement(
                            semantic_profile,
                            candidate_bindings.clone(),
                            roots,
                        )
                    };
                    let mut seed = closure_checkpoint.take();
                    let mut retained_closure_evaluations = 0;
                    let mut reopened_closure_evaluations = 0;
                    let closure = agq_kerml_semantics::close_construction_structure_with_extension(
                        current.candidate_shared(),
                        Default::default(),
                        |overlay| {
                            let context = systems_overlay_context(
                                overlay,
                                &publication,
                                current.roots(),
                                &dependency_contract,
                            )?
                            .with_producer_registry_digest(registry.digest())
                            .map_err(agq_kerml_semantics::PublicationOverlayError::Context)?;
                            if let Some(checkpoint) = seed.take() {
                                let rebound = checkpoint.rebind(&context, &registry).map_err(
                                    agq_kerml_semantics::PublicationOverlayError::Context,
                                )?;
                                retained_closure_evaluations = rebound.retained_evaluations;
                                reopened_closure_evaluations = rebound.reopened_evaluations;
                                closure_rebindings += 1;
                                total_retained_closure_evaluations += rebound.retained_evaluations;
                                total_reopened_closure_evaluations += rebound.reopened_evaluations;
                                context
                                    .with_producer_closure(rebound.certificate)
                                    .map_err(agq_kerml_semantics::PublicationOverlayError::Context)
                            } else {
                                Ok(context)
                            }
                        },
                        &extension,
                        &mut batch_progress,
                        &mut producer_progress,
                    )
                    .map_err(LibraryLoadError::ProducerClosure)?;
                    if let Some(certificate) = &closure.certificate {
                        let context = systems_overlay_context(
                            &closure.overlay,
                            &publication,
                            current.roots(),
                            &dependency_contract,
                        )
                        .map_err(LibraryLoadError::ProducerClosure)?
                        .with_producer_registry_digest(registry.digest())
                        .map_err(|error| LibraryLoadError::Interpretation(format!("{error:?}")))?;
                        closure_checkpoint =
                            Some(certificate.checkpoint(&context).map_err(|error| {
                                LibraryLoadError::Interpretation(format!("{error:?}"))
                            })?);
                    }
                    production = Some(SystemsConstructionProduction {
                        final_predicates,
                        completeness: closure.completeness,
                        converged: closure.converged,
                        stages: closure.stages,
                        counters: closure.counters,
                        retained_closure_evaluations,
                        reopened_closure_evaluations,
                        closure_rebindings,
                        total_retained_closure_evaluations,
                        total_reopened_closure_evaluations,
                    });
                    current.set_semantic_candidate(closure.overlay);
                    current.set_producer_closure(closure.certificate);
                    Ok(current)
                },
                |current| {
                    Ok(systems_candidate_queries(
                        current,
                        &publication,
                        BTreeSet::new(),
                        &dependency_contract,
                    )?
                    .status_queries())
                },
                ReferenceRefinementStrategy::DependencyDriven,
                &mut progress,
            )?;
            // Scoped preflights must exercise final predicates even when the
            // positive bootstrap happened to resolve every declared endpoint.
            if draft.candidate().obligations().is_empty() && (paths.is_none() || final_predicates) {
                break;
            }
        }
    }
    Ok(SystemsLibraryCandidate {
        draft,
        documents,
        publication,
        source_content_set: sources.content_set_id().into(),
        syntax_profile: profile,
        production,
        dependency_contract,
    })
}

fn systems_overlay_context<'m>(
    overlay: &'m agq_kernel::derived::ConstructionOverlay,
    publication: &CanonicalKermlStandardLibraries,
    roots: &[ElementId],
    contract: &agq_sysml_semantics::SysmlDependencyContract,
) -> Result<agq_kerml_semantics::SemanticContext<'m>, agq_kerml_semantics::PublicationOverlayError>
{
    publication
        .complete_overlay()
        .project_construction_overlay_context(overlay, roots, BTreeSet::new(), BTreeSet::new())
        .and_then(|context| {
            context.with_naming_extension(
                agq_sysml_semantics::SYSML_SEMANTIC_CONTEXT_DOMAIN,
                contract.context_identity_digest(),
                Arc::new(agq_sysml_semantics::SysmlNamingExtension),
            )
        })
        .map_err(agq_kerml_semantics::PublicationOverlayError::Context)
}

fn systems_candidate_queries<'m>(
    draft: &'m LibraryDraft,
    publication: &CanonicalKermlStandardLibraries,
    pending: BTreeSet<ElementId>,
    dependency_contract: &agq_sysml_semantics::SysmlDependencyContract,
) -> Result<KerMlQueries<'m>, LibraryLoadError> {
    Ok(KerMlQueries::new(systems_candidate_context(
        draft,
        publication,
        pending,
        dependency_contract,
    )?))
}

fn systems_producer_registry(
    profile: agq_kerml::BaselineProfile,
) -> agq_kerml_semantics::ProducerRegistry {
    agq_kerml_semantics::ProducerRegistry::new(
        agq_kerml_semantics::ProducerFamily::ALL
            .into_iter()
            .map(|family| family.descriptor(profile))
            .chain(agq_sysml_semantics::sysml_producer_descriptors()),
    )
    .expect("combined producer identities are unique")
}

fn systems_candidate_context<'m>(
    draft: &'m LibraryDraft,
    publication: &CanonicalKermlStandardLibraries,
    pending: BTreeSet<ElementId>,
    dependency_contract: &agq_sysml_semantics::SysmlDependencyContract,
) -> Result<agq_kerml_semantics::SemanticContext<'m>, LibraryLoadError> {
    let context = if let Some(overlay) = draft.semantic_candidate() {
        publication
            .complete_overlay()
            .project_construction_overlay_context(overlay, draft.roots(), BTreeSet::new(), pending)
    } else {
        publication.complete_overlay().project_construction_context(
            draft.candidate(),
            draft.roots(),
            BTreeSet::new(),
            pending,
        )
    }
    .and_then(|context| {
        context.with_naming_extension(
            agq_sysml_semantics::SYSML_SEMANTIC_CONTEXT_DOMAIN,
            dependency_contract.context_identity_digest(),
            Arc::new(agq_sysml_semantics::SysmlNamingExtension),
        )
    })
    .map_err(|error| LibraryLoadError::Interpretation(format!("{error:?}")))?;
    // Establish the registry independently of supplied evidence. Even the
    // initial declared refinement can certify zero-writer populations without
    // scheduling an otherwise useless producer round.
    let registry = systems_producer_registry(publication.profile());
    let context = context
        .with_producer_registry_digest(registry.digest())
        .map_err(|error| LibraryLoadError::Interpretation(format!("{error:?}")))?;
    let certificate = if let Some(certificate) = draft.producer_closure() {
        certificate.clone()
    } else {
        Arc::new(
            agq_kerml_semantics::ProducerClosureCertificate::initial(&context, &registry)
                .map_err(|error| LibraryLoadError::Interpretation(format!("{error:?}")))?,
        )
    };
    context
        .with_producer_closure(certificate)
        .map_err(|error| LibraryLoadError::Interpretation(format!("{error:?}")))
}

#[derive(Debug)]
pub(crate) struct SourceModel {
    snapshot: Snapshot,
    root: ElementId,
    publication: Arc<CanonicalKermlStandardLibraries>,
    references: Vec<ReferenceAssertion>,
    diagnostics: Vec<FrontendDiagnostic>,
    source_map: LibrarySourceMap,
    effective: Option<Box<source::EffectiveSourceModel>>,
}
impl SourceModel {
    pub(crate) fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }
    pub(crate) fn semantic_model(&self) -> &ModelView {
        self.effective
            .as_ref()
            .map_or_else(|| self.snapshot.model(), |effective| effective.model())
    }
    pub(crate) fn root(&self) -> ElementId {
        self.root
    }
    pub(crate) fn references(&self) -> &[ReferenceAssertion] {
        &self.references
    }
    pub(crate) fn diagnostics(&self) -> &[FrontendDiagnostic] {
        &self.diagnostics
    }
    pub(crate) fn source_map(&self) -> &LibrarySourceMap {
        &self.source_map
    }
    pub(crate) fn queries(&self) -> KerMlQueries<'_> {
        if let Some(effective) = &self.effective {
            return KerMlQueries::new(effective.context(self.root));
        }
        KerMlQueries::new(
            self.publication
                .complete_overlay()
                .project_context(&self.snapshot, self.root, BTreeSet::new(), BTreeSet::new())
                .expect("protected immutable publication"),
        )
    }
    pub(crate) fn sysml_queries(&self) -> Option<agq_sysml_semantics::SysmlQueries<'_>> {
        self.effective
            .as_ref()
            .map(|effective| effective.queries(self.root))
    }
    pub(crate) fn producer_status(&self) -> Option<&source::AuthoredProducerStatus> {
        self.effective.as_ref().map(|effective| &effective.status)
    }
    pub(crate) fn producer_closure(
        &self,
    ) -> Option<&Arc<agq_kerml_semantics::ProducerClosureCertificate>> {
        self.effective
            .as_ref()
            .and_then(|effective| effective.certificate.as_ref())
    }
}
pub use source::AuthoredProducerStatus;

fn base(publication: &CanonicalKermlStandardLibraries) -> Result<Snapshot, LibraryLoadError> {
    let registry = Arc::new(
        agq_sysml::registry_for_profile(publication.profile())
            .map_err(|error| LibraryLoadError::Interpretation(error.to_string()))?,
    );
    Ok(Snapshot::with_immutable_dependency_in_registry(
        publication
            .project_snapshot()
            .immutable_dependency()
            .expect("accepted dependency")
            .clone(),
        registry,
    )?)
}

pub(crate) fn lower_source(
    inputs: &[SourceInput<'_>],
    previous: Option<&SourceModel>,
    root: ElementId,
    origin: DeclaredOrigin,
    publication: Arc<CanonicalKermlStandardLibraries>,
) -> Result<SourceModel, LibraryLoadError> {
    let base = base(&publication)?;
    let draft = library::refinement::refine(
        |resolved| {
            construction::construct_on(
                inputs,
                resolved,
                publication.profile(),
                base.clone(),
                Some((root, origin.clone())),
            )
        },
        |draft| {
            Ok(KerMlQueries::new(
                publication
                    .complete_overlay()
                    .project_construction_context(
                        draft.candidate(),
                        &[root],
                        BTreeSet::new(),
                        BTreeSet::new(),
                    )
                    .map_err(|error| LibraryLoadError::Interpretation(format!("{error:?}")))?,
            )
            .status_queries())
        },
        ReferenceRefinementStrategy::DependencyDriven,
        |_| {},
    )?;
    let desired = draft.strict_snapshot()?;
    let snapshot = if let Some(previous) = previous {
        crate::lowering::publish(previous.snapshot(), &desired)?
    } else {
        desired
    };
    let q = KerMlQueries::new(
        publication
            .complete_overlay()
            .project_context(&snapshot, root, BTreeSet::new(), BTreeSet::new())
            .map_err(|error| LibraryLoadError::Interpretation(format!("{error:?}")))?,
    );
    let (references, diagnostics) =
        source_references(inputs, draft.references(), draft.source_map(), root, &q)?;
    drop(q);
    Ok(SourceModel {
        snapshot,
        root,
        publication,
        references,
        diagnostics,
        source_map: draft.source_map().clone(),
        effective: None,
    })
}

fn source_references(
    inputs: &[SourceInput<'_>],
    pending_references: &[library::PendingLibraryReference],
    source_map: &LibrarySourceMap,
    root: ElementId,
    q: &KerMlQueries<'_>,
) -> Result<(Vec<ReferenceAssertion>, Vec<FrontendDiagnostic>), LibraryLoadError> {
    let model = q.model();
    let mut references = Vec::new();
    let mut diagnostics = Vec::new();
    for reference in pending_references {
        let source = &source_map[&FactKey::Element(reference.relationship)];
        let alias_source = inputs
            .iter()
            .find(|input| input.syntax.document() == source.document)
            .and_then(|input| {
                input
                    .syntax
                    .nodes()
                    .find(|node| Some(node.id()) == source.syntax_node)
            })
            .is_some_and(|node| node.kind() == production::Production::AliasMember);
        let (kind, alias, visibility) =
            reference_metadata(model, reference.relationship, alias_source)?;
        let specific = q
            .owning_related_element(reference.relationship)
            .value
            .unwrap_or(root);
        let answer = q.lookup_relationship_target(
            reference.relationship,
            reference.property,
            &reference.name,
        );
        let complete = answer.completeness == Completeness::Complete;
        let resolution = answer.map(|matches| {
            if !complete {
                return Resolution::Incomplete;
            }
            let ids: Vec<_> = matches
                .into_iter()
                .map(|member| {
                    if reference.membership_target {
                        member.membership
                    } else {
                        member.element
                    }
                })
                .collect();
            match ids.as_slice() {
                [] => Resolution::Unresolved,
                [id] => {
                    let actual = model.element(*id).expect("resolved element").metaclass();
                    if model
                        .registry()
                        .is_subtype(actual, reference.expected)
                        .unwrap_or(false)
                    {
                        Resolution::Resolved(*id)
                    } else {
                        Resolution::WrongKind(*id)
                    }
                }
                _ => Resolution::Ambiguous(ids),
            }
        });
        let stored = model
            .navigation_slot(reference.relationship, reference.property)
            .and_then(|slot| {
                slot.value().values().find_map(|value| match value {
                    agq_kernel::value::Value::Reference(id) => Some(*id),
                    _ => None,
                })
            });
        if !matches!(resolution.value, Resolution::Resolved(id) if Some(id) == stored) {
            diagnostics.push(FrontendDiagnostic {
                domain: FrontendDiagnosticDomain::Resolution,
                code: "SYSML_SOURCE_REFERENCE",
                origin: reference.origin.clone(),
                message: format!(
                    "Canonical reference is {:?}; stored endpoint {stored:?}",
                    resolution.value
                ),
            });
        }
        references.push(ReferenceAssertion {
            relationship: reference.relationship,
            specific,
            kind,
            name: reference.name.clone(),
            origin: reference.origin.clone(),
            resolution,
            alias,
            visibility,
        });
    }
    Ok((references, diagnostics))
}

fn reference_metadata(
    model: &ModelView,
    relationship: ElementId,
    alias_source: bool,
) -> Result<(ReferenceKind, Option<String>, Visibility), LibraryLoadError> {
    let class = model
        .element(relationship)
        .expect("canonical relationship")
        .metaclass();
    let is = |expected| {
        model
            .registry()
            .is_subtype(class, expected)
            .unwrap_or(false)
    };
    let kind = if is(c::FEATURE_TYPING) {
        ReferenceKind::Typing
    } else if is(c::REDEFINITION) {
        ReferenceKind::Redefinition
    } else if is(c::SUBSETTING) {
        ReferenceKind::Subsetting
    } else if is(c::SPECIALIZATION) {
        ReferenceKind::Specialization
    } else if is(c::MEMBERSHIP_IMPORT) {
        ReferenceKind::MembershipImport
    } else if is(c::NAMESPACE_IMPORT) {
        ReferenceKind::NamespaceImport
    } else if class == c::MEMBERSHIP && alias_source {
        ReferenceKind::Alias
    } else {
        return Err(LibraryLoadError::Interpretation(format!(
            "Authored source reference metadata for metaclass {class} is not implemented"
        )));
    };
    let alias = if kind == ReferenceKind::Alias {
        model
            .navigation_slot(relationship, p::MEMBERSHIP_MEMBER_NAME)
            .and_then(|slot| {
                slot.value().values().find_map(|value| {
                    if let Value::String(name) = value {
                        Some(name.clone())
                    } else {
                        None
                    }
                })
            })
    } else {
        None
    };
    let property = if is(c::IMPORT) {
        Some(p::IMPORT_VISIBILITY)
    } else if is(c::MEMBERSHIP) {
        Some(p::MEMBERSHIP_VISIBILITY)
    } else {
        None
    };
    let visibility = if let Some(property) = property {
        let literal = model
            .navigation_slot(relationship, property)
            .and_then(|slot| {
                slot.value().values().find_map(|value| {
                    if let Value::Enumeration(id) = value {
                        Some(*id)
                    } else {
                        None
                    }
                })
            })
            .ok_or_else(|| {
                LibraryLoadError::Interpretation("Reference visibility is unavailable".into())
            })?;
        let agq_kernel::metamodel::ValueKind::Enumeration(domain) = model
            .registry()
            .property(property)
            .expect("visibility descriptor")
            .value_kind
        else {
            unreachable!("pinned visibility enumeration")
        };
        match model
            .registry()
            .enumeration(domain)
            .expect("visibility domain")
            .literals
            .get(&literal)
            .map(String::as_str)
        {
            Some("public") => Visibility::Public,
            Some("protected") => Visibility::Protected,
            Some("private") => Visibility::Private,
            _ => {
                return Err(LibraryLoadError::Interpretation(
                    "Reference visibility literal is invalid".into(),
                ));
            }
        }
    } else {
        Visibility::Public
    };
    Ok((kind, alias, visibility))
}
