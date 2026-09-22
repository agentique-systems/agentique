//! Authored language integration over the two accepted, immutable publications.
use super::*;
use agq_kerml_semantics::{
    ProducerClosedDependency, ProducerClosureCertificate, ProducerFamily, ProducerRegistry,
    PublicationCounters, PublicationOverlayError, PublicationStage, SemanticContext,
};
use agq_kernel::derived::DerivedOverlay;
use agq_sysml_semantics::{SysmlProducerExtension, SysmlQueries, SysmlSemanticContext};

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
    mounted: Arc<ProducerClosedDependency>,
}
impl AcceptedSourceDependency {
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
    fn registry(&self) -> ProducerRegistry {
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
    fn candidate_queries<'m>(
        &self,
        draft: &'m LibraryDraft,
        root: ElementId,
    ) -> Result<KerMlQueries<'m>, LibraryLoadError> {
        Ok(KerMlQueries::new(self.candidate_context(draft, root)?))
    }
    fn candidate_context<'m>(
        &self,
        draft: &'m LibraryDraft,
        root: ElementId,
    ) -> Result<SemanticContext<'m>, LibraryLoadError> {
        let context = if let Some(overlay) = draft.semantic_candidate() {
            self.mounted.project_construction_overlay_context(
                overlay,
                &[root],
                BTreeSet::new(),
                BTreeSet::new(),
            )
        } else {
            self.mounted.project_construction_context(
                draft.candidate(),
                &[root],
                BTreeSet::new(),
                BTreeSet::new(),
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
    fn extension(&self, root: ElementId, final_predicates: bool) -> SysmlProducerExtension {
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

pub(crate) fn lower_accepted_source(
    inputs: &[SourceInput<'_>],
    previous: Option<&SourceModel>,
    root: ElementId,
    origin: DeclaredOrigin,
    dependency: Arc<AcceptedSourceDependency>,
) -> Result<SourceModel, LibraryLoadError> {
    let base = dependency.mounted.project_snapshot();
    let profile = dependency.publication.accepted_kerml().profile();
    let registry = dependency.registry();
    let mut retained_evaluations = 0;
    let mut reopened_evaluations = 0;
    let mut draft = library::refinement::refine(
        |resolved| {
            construction::construct_on(
                inputs,
                resolved,
                profile,
                base.clone(),
                Some((root, origin.clone())),
            )
        },
        |draft| Ok(dependency.candidate_queries(draft, root)?.status_queries()),
        ReferenceRefinementStrategy::DependencyDriven,
        |_| {},
    )?;
    // A missing endpoint may require an inherited or producer-created member.
    // Keep these consequences separate from canonical declared construction.
    if !draft.candidate().obligations().is_empty() {
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
                    let mut draft = construction::construct_on(
                        inputs,
                        resolved,
                        profile,
                        base.clone(),
                        Some((root, origin.clone())),
                    )?;
                    let mut seed = checkpoint.take();
                    let closed = agq_kerml_semantics::close_construction_structure_with_extension(
                        draft.candidate_shared(),
                        Default::default(),
                        |overlay| {
                            let context = dependency
                                .mounted
                                .project_construction_overlay_context(
                                    overlay,
                                    &[root],
                                    BTreeSet::new(),
                                    BTreeSet::new(),
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
                    if let Some(certificate) = &closed.certificate {
                        let context = dependency
                            .mounted
                            .project_construction_overlay_context(
                                &closed.overlay,
                                &[root],
                                BTreeSet::new(),
                                BTreeSet::new(),
                            )
                            .map_err(interpretation)?;
                        checkpoint =
                            Some(certificate.checkpoint(&context).map_err(interpretation)?);
                    }
                    draft.set_semantic_candidate(closed.overlay);
                    draft.set_producer_closure(closed.certificate);
                    Ok(draft)
                },
                |draft| Ok(dependency.candidate_queries(draft, root)?.status_queries()),
                ReferenceRefinementStrategy::DependencyDriven,
                |_| {},
            )?;
            if draft.candidate().obligations().is_empty() {
                break;
            }
        }
    }
    let desired = draft.strict_snapshot()?;
    let snapshot = if let Some(previous) = previous {
        crate::lowering::publish(previous.snapshot(), &desired)?
    } else {
        desired
    };
    // Final strict reconstruction changes graph identity. Retain only evaluations
    // whose actual semantic reads survive the checked delta. Prior revisions are
    // immutable; no producer result is copied into declared source records.
    let mut seed = if let Some(certificate) = draft.producer_closure() {
        let context = dependency.candidate_context(&draft, root)?;
        Some(certificate.checkpoint(&context).map_err(interpretation)?)
    } else if let Some(effective) = previous.and_then(|previous| previous.effective.as_ref()) {
        effective
            .certificate
            .as_ref()
            .map(|certificate| certificate.checkpoint(&effective.context(root)))
            .transpose()
            .map_err(interpretation)?
    } else {
        None
    };
    let closed = agq_kerml_semantics::close_result_structure_with_extension(
        &snapshot,
        Default::default(),
        |overlay| {
            let context = dependency
                .mounted
                .project_overlay_context(overlay, &[root])
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
        &dependency.extension(root, true),
        |_, _, _, _| {},
        |_| {},
    )
    .map_err(LibraryLoadError::ProducerClosure)?;
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
    let queries = KerMlQueries::new(effective.context(root));
    let (references, diagnostics) = source_references(inputs, &draft, root, &queries)?;
    drop(queries);
    Ok(SourceModel {
        snapshot,
        root,
        publication: dependency.publication.accepted_kerml().clone(),
        references,
        diagnostics,
        source_map: draft.source_map().clone(),
        effective: Some(Box::new(effective)),
    })
}
