//! Authored language integration over the two accepted, immutable publications.
use super::*;
use crate::{CompilationControl, CompilationStage};
use agq_kerml_semantics::{
    ProducerClosedDependency, ProducerClosureCertificate, ProducerFamily, ProducerRegistry,
    PublicationCounters, PublicationOverlayError, PublicationStage, SemanticContext,
};
use agq_kernel::derived::DerivedOverlay;
use agq_sysml_semantics::{SysmlProducerExtension, SysmlQueries, SysmlSemanticContext};
use std::collections::BTreeMap;
use std::time::Instant;

/// Result of the combined authored producer scheduler, independently of syntax
/// and mandatory reference diagnostics. This is not full language validation.
#[derive(Clone, Debug)]
pub struct AuthoredProducerStatus {
    pub completeness: Completeness,
    pub converged: bool,
    pub stages: Vec<PublicationStage>,
    pub counters: PublicationCounters,
    /// Graph reconstruction revalidates prior reads; these counts never replace
    /// the scheduler's actual final completeness or convergence result.
    pub retained_evaluations: usize,
    pub reopened_evaluations: usize,
}

/// Minted once per source history from the accepted facade, never from an
/// arbitrary caller-supplied graph or a claimed acceptance flag.
#[derive(Debug)]
pub(crate) struct AcceptedSourceDependency {
    pub(crate) publication: Arc<CanonicalSysmlSystemsLibrary>,
    pub(crate) mounted: Arc<ProducerClosedDependency>,
}
impl AcceptedSourceDependency {
    pub(crate) fn syntax_profile(&self) -> production::SysmlSyntaxProfile {
        match self.publication.identity().dependencies.sysml_profile {
            agq_sysml_semantics::SysmlBaselineProfile::Published => {
                production::SysmlSyntaxProfile::Published
            }
            agq_sysml_semantics::SysmlBaselineProfile::OperationalV1 => {
                production::SysmlSyntaxProfile::OperationalV1
            }
            agq_sysml_semantics::SysmlBaselineProfile::OperationalV2 => {
                production::SysmlSyntaxProfile::OperationalV2
            }
            agq_sysml_semantics::SysmlBaselineProfile::OperationalV3 => {
                production::SysmlSyntaxProfile::OperationalV3
            }
        }
    }
    pub(crate) fn new(
        publication: Arc<CanonicalSysmlSystemsLibrary>,
    ) -> Result<Arc<Self>, LibraryLoadError> {
        let mounted = publication
            .producer_closed_dependency()
            .map_err(interpretation)?;
        Ok(Arc::new(Self {
            publication,
            mounted,
        }))
    }
    pub(crate) fn registry(&self) -> ProducerRegistry {
        ProducerRegistry::new(
            ProducerFamily::ALL
                .into_iter()
                .map(|family| family.descriptor(self.publication.accepted_kerml().profile()))
                .chain(agq_sysml_semantics::sysml_producer_descriptors()),
        )
        .expect("registered combined language producer identities")
    }
    fn initial<'m>(
        &self,
        context: SemanticContext<'m>,
    ) -> Result<SemanticContext<'m>, LibraryLoadError> {
        let certificate = Arc::new(
            ProducerClosureCertificate::initial(&context, &self.registry())
                .map_err(interpretation)?,
        );
        context
            .with_producer_closure(certificate)
            .map_err(interpretation)
    }
    fn candidate_context<'m>(
        &self,
        draft: &'m LibraryDraft,
        root: ElementId,
    ) -> Result<SemanticContext<'m>, LibraryLoadError> {
        self.candidate_context_with_pending(draft, root, &BTreeSet::new())
    }
    pub(crate) fn candidate_context_with_pending<'m>(
        &self,
        draft: &'m LibraryDraft,
        root: ElementId,
        pending: &BTreeSet<ElementId>,
    ) -> Result<SemanticContext<'m>, LibraryLoadError> {
        let context = if let Some(overlay) = draft.semantic_candidate() {
            self.mounted.project_construction_overlay_context(
                overlay,
                &[root],
                BTreeSet::new(),
                pending.clone(),
            )
        } else {
            self.mounted.project_construction_context(
                draft.candidate(),
                &[root],
                BTreeSet::new(),
                pending.clone(),
            )
        }
        .map_err(interpretation)?;
        let context = if let Some(certificate) = draft.producer_closure() {
            context
                .with_producer_closure(certificate.clone())
                .map_err(interpretation)?
        } else {
            self.initial(context)?
        };
        Ok(context)
    }
    pub(crate) fn extension(
        &self,
        root: ElementId,
        final_predicates: bool,
    ) -> SysmlProducerExtension {
        let roots = std::iter::once(root)
            .chain(self.publication.roots().iter().copied())
            .chain(self.publication.accepted_kerml().roots().iter().copied())
            .collect();
        let profile = self.publication.identity().dependencies.sysml_profile;
        let bindings = self.publication.bindings().clone();
        if final_predicates {
            SysmlProducerExtension::new(profile, bindings, roots)
        } else {
            SysmlProducerExtension::for_reference_refinement(profile, bindings, roots)
        }
    }
}

#[derive(Debug)]
pub(super) struct EffectiveSourceModel {
    dependency: Arc<AcceptedSourceDependency>,
    overlay: DerivedOverlay,
    pub(super) certificate: Option<Arc<ProducerClosureCertificate>>,
    pub(super) status: AuthoredProducerStatus,
}
impl EffectiveSourceModel {
    #[cfg(feature = "verification")]
    pub(super) fn dependency_storage(&self) -> agq_kernel::storage_observer::DependencyStorage {
        agq_kernel::storage_observer::overlay_storage(&self.overlay)
    }
    pub(super) fn model(&self) -> &ModelView {
        self.overlay.model()
    }
    pub(super) fn context(&self, root: ElementId) -> SemanticContext<'_> {
        let context = self
            .dependency
            .mounted
            .project_overlay_context(&self.overlay, &[root])
            .expect("exact shared authored dependency");
        match &self.certificate {
            Some(certificate) => context
                .with_producer_closure(certificate.clone())
                .expect("exact final authored frontier"),
            None => context,
        }
    }
    pub(super) fn queries(&self, root: ElementId) -> SysmlQueries<'_> {
        let accepted = &self.dependency.publication;
        let context = SysmlSemanticContext::for_closed_dependency(
            self.context(root),
            &accepted.identity().dependencies,
            accepted.bindings().clone(),
        )
        .expect("immutable authenticated Systems binding identities");
        SysmlQueries::new(context)
    }
}

fn interpretation(error: impl std::fmt::Debug) -> LibraryLoadError {
    LibraryLoadError::Interpretation(format!("{error:?}"))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn prepare_accepted_source(
    inputs: &[SourceInput<'_>],
    root: ElementId,
    origin: DeclaredOrigin,
    dependency: Arc<AcceptedSourceDependency>,
    pending: &BTreeSet<ElementId>,
    history: Option<&agq_kernel::DeclaredConstructionHistory>,
    cache: Option<&std::cell::RefCell<construction::LoweringCache>>,
    timings: Option<&std::cell::RefCell<crate::CompilationTimings>>,
    control: &CompilationControl,
) -> Result<PreparedSource, LibraryLoadError> {
    control.check()?;
    let base = dependency.mounted.project_snapshot();
    let profile = dependency.publication.accepted_kerml().profile();
    let registry = dependency.registry();
    let mut retained_evaluations = 0;
    let mut reopened_evaluations = 0;
    let mut status = None;
    let construct = |resolved: &BTreeMap<(ElementId, agq_kernel::PropertyId), ElementId>| {
        control.enter(CompilationStage::DeclaredModel)?;
        let started = Instant::now();
        let mut cache = cache.map(std::cell::RefCell::borrow_mut);
        let draft = construction::construct_on_cached(
            inputs,
            resolved,
            profile,
            base.clone(),
            Some((root, origin.clone())),
            cache.as_deref_mut(),
        )?;
        control.check()?;
        let draft = if let Some(history) = history {
            draft.reconcile_declared(history)?.1
        } else {
            draft
        };
        if let Some(timings) = timings {
            timings.borrow_mut().declared_construction_micros += crate::elapsed_micros(started);
        }
        Ok(draft)
    };
    let mut draft = library::refinement::refine(
        &construct,
        |draft| {
            control.enter(CompilationStage::Resolving)?;
            Ok(
                KerMlQueries::new(dependency.candidate_context_with_pending(draft, root, pending)?)
                    .status_queries(),
            )
        },
        ReferenceRefinementStrategy::DependencyDriven,
        |round| record_refinement_timing(timings, round),
    )?;
    // A missing endpoint may require an inherited or producer-created member.
    // Keep these consequences separate from canonical declared construction.
    if !draft.candidate().obligations().is_empty() || !pending.is_empty() {
        for final_predicates in [false, true] {
            let mut checkpoint: Option<agq_kerml_semantics::ProducerClosureCheckpoint> = None;
            let endpoints = draft
                .references()
                .iter()
                .filter_map(|reference| {
                    draft
                        .candidate()
                        .model()
                        .navigation_slot(reference.relationship, reference.property)
                        .and_then(|slot| {
                            slot.value().values().find_map(|value| match value {
                                Value::Reference(id) => {
                                    Some(((reference.relationship, reference.property), *id))
                                }
                                _ => None,
                            })
                        })
                })
                .collect();
            drop(draft);
            let extension = dependency.extension(root, final_predicates);
            draft = library::refinement::refine_from(
                endpoints,
                |resolved| {
                    let mut draft = construct(resolved)?;
                    control.enter(CompilationStage::SemanticClosure)?;
                    let started = Instant::now();
                    let mut seed = checkpoint.take();
                    let closed = agq_kerml_semantics::close_construction_structure_with_extension(
                        draft.candidate_shared(),
                        agq_kerml_semantics::PublicationClosureOptions {
                            cancellation: control.cancellation(),
                            ..Default::default()
                        },
                        |overlay| {
                            let context = dependency
                                .mounted
                                .project_construction_overlay_context(
                                    overlay,
                                    &[root],
                                    BTreeSet::new(),
                                    pending.clone(),
                                )
                                .map_err(PublicationOverlayError::Context)?;
                            if let Some(previous) = seed.take() {
                                let rebound = previous
                                    .rebind(&context, &registry)
                                    .map_err(PublicationOverlayError::Context)?;
                                retained_evaluations += rebound.retained_evaluations;
                                reopened_evaluations += rebound.reopened_evaluations;
                                context
                                    .with_producer_closure(rebound.certificate)
                                    .map_err(PublicationOverlayError::Context)
                            } else {
                                Ok(context)
                            }
                        },
                        &extension,
                        |_, _, _, _| {},
                        |_| {},
                    )
                    .map_err(LibraryLoadError::ProducerClosure)?;
                    control.check()?;
                    if let Some(cache) = cache {
                        cache.borrow_mut().preparatory_producer_subjects_evaluated +=
                            closed.counters.subjects_evaluated;
                    }
                    if let Some(certificate) = &closed.certificate {
                        let context = dependency
                            .mounted
                            .project_construction_overlay_context(
                                &closed.overlay,
                                &[root],
                                BTreeSet::new(),
                                pending.clone(),
                            )
                            .map_err(interpretation)?;
                        checkpoint =
                            Some(certificate.checkpoint(&context).map_err(interpretation)?);
                    }
                    status = Some(AuthoredProducerStatus {
                        completeness: closed.completeness,
                        converged: closed.converged,
                        stages: closed.stages,
                        counters: closed.counters,
                        retained_evaluations,
                        reopened_evaluations,
                    });
                    draft.set_semantic_candidate(closed.overlay);
                    draft.set_producer_closure(closed.certificate);
                    if let Some(timings) = timings {
                        timings.borrow_mut().preparatory_producers_micros +=
                            crate::elapsed_micros(started);
                    }
                    Ok(draft)
                },
                |draft| {
                    control.enter(CompilationStage::Resolving)?;
                    Ok(KerMlQueries::new(
                        dependency.candidate_context_with_pending(draft, root, pending)?,
                    )
                    .status_queries())
                },
                ReferenceRefinementStrategy::DependencyDriven,
                |round| record_refinement_timing(timings, round),
            )?;
            if draft.candidate().obligations().is_empty() {
                break;
            }
        }
    }
    control.check()?;
    Ok(PreparedSource {
        draft,
        status,
        retained_evaluations,
        reopened_evaluations,
    })
}

fn record_refinement_timing(
    timings: Option<&std::cell::RefCell<crate::CompilationTimings>>,
    round: &library::refinement::ReferenceRefinementRound,
) {
    if let Some(timings) = timings {
        let elapsed =
            round.context_elapsed + round.change_detection_elapsed + round.resolution_elapsed;
        timings.borrow_mut().reference_refinement_micros +=
            u64::try_from(elapsed.as_micros()).unwrap_or(u64::MAX);
    }
}

pub(crate) struct PreparedSource {
    pub(crate) draft: LibraryDraft,
    pub(crate) status: Option<AuthoredProducerStatus>,
    retained_evaluations: usize,
    reopened_evaluations: usize,
}

pub(crate) fn lower_accepted_source(
    inputs: &[SourceInput<'_>],
    previous: Option<&SourceModel>,
    root: ElementId,
    origin: DeclaredOrigin,
    dependency: Arc<AcceptedSourceDependency>,
) -> Result<SourceModel, LibraryLoadError> {
    let control = CompilationControl::default();
    let prepared = prepare_accepted_source(
        inputs,
        root,
        origin,
        dependency.clone(),
        &BTreeSet::new(),
        None,
        None,
        None,
        &control,
    )?;
    finish_accepted_source(
        inputs, prepared, previous, root, dependency, None, None, None, &control,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn finish_accepted_source(
    inputs: &[SourceInput<'_>],
    prepared: PreparedSource,
    previous: Option<&SourceModel>,
    root: ElementId,
    dependency: Arc<AcceptedSourceDependency>,
    desired: Option<Snapshot>,
    semantic_cache: Option<&crate::SourceSemanticCache>,
    timings: Option<&std::cell::RefCell<crate::CompilationTimings>>,
    control: &CompilationControl,
) -> Result<SourceModel, LibraryLoadError> {
    control.enter(CompilationStage::SemanticClosure)?;
    let closure_started = Instant::now();
    let PreparedSource {
        draft,
        mut retained_evaluations,
        mut reopened_evaluations,
        ..
    } = prepared;
    let registry = dependency.registry();
    let snapshot = if let Some(snapshot) = desired {
        snapshot
    } else {
        let desired = draft.strict_snapshot()?;
        if let Some(previous) = previous {
            crate::lowering::publish(previous.snapshot(), &desired)?
        } else {
            desired
        }
    };
    // Final strict reconstruction changes graph identity. Retain only evaluations
    // whose actual semantic reads survive the checked delta. Prior revisions are
    // immutable; no producer result is copied into declared source records.
    let checkpoint_started = Instant::now();
    control.check()?;
    let mut seed = if let Some(certificate) = draft.producer_closure() {
        let context = dependency.candidate_context(&draft, root)?;
        Some(
            certificate
                .checkpoint_sharing_dependency(&context)
                .map_err(interpretation)?,
        )
    } else if let Some(effective) = previous.and_then(|previous| previous.effective.as_ref()) {
        effective
            .certificate
            .as_ref()
            .map(|certificate| certificate.checkpoint_sharing_dependency(&effective.context(root)))
            .transpose()
            .map_err(interpretation)?
    } else {
        None
    };
    if let Some(timings) = timings {
        timings.borrow_mut().closure_checkpoint_micros += crate::elapsed_micros(checkpoint_started);
    }
    control.check()?;
    // The final audit needs only source metadata. Release construction indexes,
    // any partial overlay and the temporary mount before strict closure starts.
    // The checkpoint retains semantic fingerprints, not the previous graph.
    let pending_references = draft.references().to_vec();
    let source_map = draft.source_map().clone();
    drop(draft);
    let closed = if let Some(cache) = semantic_cache {
        let overlay = cache.restore_frontier(snapshot.clone(), &dependency, root)?;
        agq_kerml_semantics::close_result_structure_on_overlay_with_extension(
            overlay,
            agq_kerml_semantics::PublicationClosureOptions {
                cancellation: control.cancellation(),
                ..Default::default()
            },
            |overlay| {
                let context = dependency
                    .mounted
                    .project_overlay_context(overlay, &[root])
                    .map_err(PublicationOverlayError::Context)?;
                // Cache bytes supply no producer acceptance. Reuse only the
                // source-derived checkpoint constructed above, after checking
                // its actual reads against this exact restored frontier. The
                // scheduler still evaluates every reopened population.
                if let Some(previous) = seed.take() {
                    let rebound_started = Instant::now();
                    let rebound = previous
                        .rebind(&context, &registry)
                        .map_err(PublicationOverlayError::Context)?;
                    if let Some(timings) = timings {
                        timings.borrow_mut().closure_rebind_micros +=
                            crate::elapsed_micros(rebound_started);
                    }
                    retained_evaluations += rebound.retained_evaluations;
                    reopened_evaluations += rebound.reopened_evaluations;
                    context
                        .with_producer_closure(rebound.certificate)
                        .map_err(PublicationOverlayError::Context)
                } else {
                    Ok(context)
                }
            },
            &dependency.extension(root, true),
            |_, _, _, _| {},
            |_| {},
        )
    } else {
        agq_kerml_semantics::close_result_structure_with_extension(
            &snapshot,
            agq_kerml_semantics::PublicationClosureOptions {
                cancellation: control.cancellation(),
                ..Default::default()
            },
            |overlay| {
                let context = dependency
                    .mounted
                    .project_overlay_context(overlay, &[root])
                    .map_err(PublicationOverlayError::Context)?;
                if let Some(previous) = seed.take() {
                    let rebound_started = Instant::now();
                    let rebound = previous
                        .rebind(&context, &registry)
                        .map_err(PublicationOverlayError::Context)?;
                    if let Some(timings) = timings {
                        timings.borrow_mut().closure_rebind_micros +=
                            crate::elapsed_micros(rebound_started);
                    }
                    retained_evaluations += rebound.retained_evaluations;
                    reopened_evaluations += rebound.reopened_evaluations;
                    context
                        .with_producer_closure(rebound.certificate)
                        .map_err(PublicationOverlayError::Context)
                } else {
                    Ok(context)
                }
            },
            &dependency.extension(root, true),
            |_, _, _, _| {},
            |_| {},
        )
    }
    .map_err(LibraryLoadError::ProducerClosure)?;
    control.check()?;
    let effective = EffectiveSourceModel {
        dependency: dependency.clone(),
        overlay: closed.overlay,
        certificate: closed.certificate,
        status: AuthoredProducerStatus {
            completeness: closed.completeness,
            converged: closed.converged,
            stages: closed.stages,
            counters: closed.counters,
            retained_evaluations,
            reopened_evaluations,
        },
    };
    if let Some(timings) = timings {
        timings.borrow_mut().final_closure_micros += crate::elapsed_micros(closure_started);
    }
    let references_started = Instant::now();
    control.enter(CompilationStage::Resolving)?;
    let queries = KerMlQueries::new(effective.context(root));
    if let Some(cache) = semantic_cache {
        cache.verify_closed(queries.context(), effective.certificate.as_deref())?;
    }
    let (references, diagnostics) =
        source_references(inputs, &pending_references, &source_map, root, &queries)?;
    drop(queries);
    if let Some(timings) = timings {
        timings.borrow_mut().final_references_micros += crate::elapsed_micros(references_started);
    }
    control.check()?;
    Ok(SourceModel {
        snapshot,
        root,
        publication: dependency.publication.accepted_kerml().clone(),
        references,
        diagnostics,
        source_map,
        effective: Some(Box::new(effective)),
    })
}

/// Only authored effective records are cached; accepted dependency bytes are
/// excluded by the kernel's versioned local-frontier archive contract.
pub(crate) fn write_source_frontier(
    model: &SourceModel,
) -> Result<Vec<u8>, agq_kernel::archive::ArchiveError> {
    let effective = model
        .effective
        .as_ref()
        .ok_or(agq_kernel::archive::ArchiveError::Invalid(
            "missing source overlay",
        ))?;
    let mut bytes = Vec::new();
    agq_kernel::archive::write_bound_frontier(
        &effective.overlay,
        crate::SourceSemanticCache::dependency_identity(&effective.dependency.publication),
        &mut bytes,
    )?;
    Ok(bytes)
}
