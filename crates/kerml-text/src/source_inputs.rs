//! Immutable authored inputs and exact Working compilation; history belongs to callers.
use super::*;
use crate::library::{LibraryDraft, LibraryLoadError, LibrarySourceMap, construction::SourceInput};
use crate::sysml::{
    AcceptedSourceDependency, AuthoredProducerStatus, CanonicalSysmlSystemsLibrary,
    SystemsPublicationAudit, SystemsPublicationFinding,
};
use agq_kerml_semantics::{Completeness, ProducerClosureCertificate};
use agq_kernel::provenance::{DeclaredOrigin, FactKey, SourceOrigin};
use agq_kernel::{ConstructionView, DeclaredConstructionHistory, DeclaredIdentitySet, ModelView};
use agq_sysml_semantics::{
    SysmlQueries, SysmlQueryResult, SysmlSemanticContext, SysmlSemanticContextId,
};
use std::collections::BTreeSet;
use std::time::Instant;
#[path = "source_checkpoint.rs"]
mod checkpoint;
pub use checkpoint::*;
#[path = "source_semantic_cache.rs"]
mod semantic_cache;
pub use semantic_cache::*;
#[path = "source_effective_audit.rs"]
mod effective_audit;

/// Exact source inputs. Applying edits shares every unchanged document and syntax arena.
#[derive(Clone, Debug)]
pub struct SourceInputs {
    project: ProjectId,
    root: ElementId,
    limits: ParseLimits,
    dependency: Arc<AcceptedSourceDependency>,
    documents: BTreeMap<String, Arc<ProjectDocument>>,
    documents_reparsed: usize,
}
impl SourceInputs {
    /// Authenticate both accepted publications once; this never produces standards.
    pub fn with_accepted_sysml(
        publication: Arc<CanonicalSysmlSystemsLibrary>,
    ) -> Result<Self, LibraryLoadError> {
        Ok(Self {
            project: ProjectId(GeneratorId::new()),
            root: ElementId::new(),
            limits: ParseLimits::default(),
            dependency: AcceptedSourceDependency::new(publication)?,
            documents: BTreeMap::new(),
            documents_reparsed: 0,
        })
    }
    /// Prepare inputs without modifying this revision. Language recovery is retained.
    pub fn apply(
        &self,
        changes: impl IntoIterator<Item = ProjectChange>,
    ) -> Result<Self, ProjectError> {
        let changes: Vec<_> = changes.into_iter().collect();
        let documents_reparsed = changes
            .iter()
            .filter(|change| {
                matches!(
                    change,
                    ProjectChange::Add { .. }
                        | ProjectChange::Replace { .. }
                        | ProjectChange::Edit { .. }
                )
            })
            .count();
        let documents = prepare_documents(
            &self.documents,
            changes,
            self.limits,
            true,
            Some(self.dependency.syntax_profile()),
        )?;
        Ok(Self {
            documents,
            documents_reparsed,
            ..self.clone()
        })
    }
    pub fn project(&self) -> ProjectId {
        self.project
    }
    pub fn root(&self) -> ElementId {
        self.root
    }
    pub fn documents(&self) -> impl Iterator<Item = (&str, &ProjectDocument)> {
        self.documents
            .iter()
            .map(|(path, document)| (path.as_str(), document.as_ref()))
    }
    pub fn document_at(&self, path: &str) -> Option<&ProjectDocument> {
        self.documents.get(path).map(Arc::as_ref)
    }
    pub fn document(&self, id: DocumentId) -> Option<&ProjectDocument> {
        self.documents
            .values()
            .find(|document| document.id() == id)
            .map(Arc::as_ref)
    }
    pub fn accepted_sysml(&self) -> &Arc<CanonicalSysmlSystemsLibrary> {
        &self.dependency.publication
    }
    /// Compile the exact current inputs, including recovered and unresolved Working states.
    /// Operational errors return no compilation. Previous graphs are never a fallback.
    pub fn compile(
        self: &Arc<Self>,
        previous: Option<&SourceCompilation>,
    ) -> Result<SourceCompilation, LibraryLoadError> {
        self.compile_with_history(previous, None, true, None)
    }
    fn compile_with_history(
        self: &Arc<Self>,
        previous: Option<&SourceCompilation>,
        restored: Option<(DeclaredConstructionHistory, LibrarySourceMap)>,
        incremental: bool,
        semantic_cache: Option<&SourceSemanticCache>,
    ) -> Result<SourceCompilation, LibraryLoadError> {
        let compilation_started = Instant::now();
        let timings = std::cell::RefCell::new(CompilationTimings::default());
        if previous.is_some_and(|previous| {
            previous.inputs.project != self.project
                || !Arc::ptr_eq(&previous.inputs.dependency, &self.dependency)
        }) {
            return Err(LibraryLoadError::Interpretation(
                "foreign source compilation history".into(),
            ));
        }
        #[cfg(feature = "verification")]
        let observation = agq_kerml_semantics::testing::ProducerObservation::start();
        let history = previous.map_or_else(
            || {
                DeclaredConstructionHistory::from_snapshot(
                    &self.dependency.mounted.project_snapshot(),
                )
            },
            |previous| previous.history.clone(),
        );
        let (history, ledger) = restored.unwrap_or_else(|| {
            (
                history,
                previous.map_or_else(BTreeMap::new, |previous| previous.identities.clone()),
            )
        });
        let (history, mut ledger) = prepare_identity_history(&self.documents, &history, ledger)?;
        let cache = std::cell::RefCell::new(previous.map_or_else(Default::default, |previous| {
            previous.lowering_cache.next_revision()
        }));
        cache.borrow_mut().allow_reuse = incremental;
        cache.borrow_mut().reconstruction_base = previous
            .filter(|_| incremental)
            .and_then(SourceCompilation::strict_snapshot)
            .cloned();
        cache.borrow_mut().retain_documents(
            &self
                .documents
                .values()
                .map(|document| (document.id(), document.revision()))
                .collect(),
        );
        let mut diagnostics = Vec::new();
        let mut omitted = BTreeSet::new();
        for document in self.documents.values() {
            if document.status() != DocumentStatus::Parsed {
                omitted.insert(document.id());
                diagnostics.push(SourceDiagnostic::Syntax {
                    document: document.id(),
                    revision: document.revision(),
                    status: document.status(),
                });
            }
        }
        // Unsupported source has an explicit typed emission site. Internal
        // interpretation, dependency and kernel errors are never swallowed here.
        timings.borrow_mut().identity_preparation_micros = elapsed_micros(compilation_started);
        let preparation_started = Instant::now();
        let (prepared, pending) = loop {
            let inputs = self.lowering_inputs(&omitted);
            let pending = if omitted.is_empty() {
                BTreeSet::new()
            } else {
                BTreeSet::from([self.root])
            };
            match crate::sysml::source::prepare_accepted_source(
                &inputs,
                self.root,
                DeclaredOrigin::Generated {
                    generator: self.project.0,
                },
                self.dependency.clone(),
                &pending,
                Some(&history),
                Some(&cache),
                Some(&timings),
            ) {
                Ok(prepared) => break (prepared, pending),
                Err(LibraryLoadError::UnsupportedSource { origin, construct }) => {
                    if !omitted.insert(origin.document) {
                        return Err(LibraryLoadError::UnsupportedSource { origin, construct });
                    }
                    diagnostics.push(SourceDiagnostic::Unsupported {
                        origin: *origin,
                        construct,
                    });
                }
                Err(error) => return Err(error),
            }
        };
        timings.borrow_mut().source_preparation_micros = elapsed_micros(preparation_started);
        let (history, _) = history.reconcile(prepared.draft.candidate().clone())?;
        for (fact, origin) in prepared.draft.source_map() {
            if matches!(
                fact,
                FactKey::Element(_) | FactKey::AssociationOccurrence(_)
            ) {
                ledger.insert(*fact, origin.clone());
            }
        }
        let inputs = self.lowering_inputs(&omitted);
        let frontier = if pending.is_empty() && prepared.draft.candidate().obligations().is_empty()
        {
            let validation_started = Instant::now();
            let snapshot = prepared.draft.candidate().clone().revalidate_declared()?;
            timings.borrow_mut().strict_kernel_validation_micros =
                elapsed_micros(validation_started);
            let previous = previous.and_then(|previous| match &previous.frontier {
                SourceFrontier::Strict(model) => Some(model.as_ref()),
                _ => None,
            });
            let model = crate::sysml::source::finish_accepted_source(
                &inputs,
                prepared,
                previous,
                self.root,
                self.dependency.clone(),
                Some(snapshot),
                semantic_cache,
                Some(&timings),
            )?;
            diagnostics.extend(
                model
                    .diagnostics()
                    .iter()
                    .cloned()
                    .map(SourceDiagnostic::Frontend),
            );
            SourceFrontier::Strict(Box::new(model))
        } else {
            if semantic_cache.is_some() {
                return Err(LibraryLoadError::Interpretation(
                    "semantic cache requires strict source declarations".into(),
                ));
            }
            let q = KerMlQueries::new(self.dependency.candidate_context_with_pending(
                &prepared.draft,
                self.root,
                &pending,
            )?);
            let (references, reference_diagnostics) = crate::sysml::source_references(
                &inputs,
                prepared.draft.references(),
                prepared.draft.source_map(),
                self.root,
                &q,
            )?;
            drop(q);
            diagnostics.extend(
                reference_diagnostics
                    .into_iter()
                    .map(SourceDiagnostic::Frontend),
            );
            diagnostics.extend(
                prepared
                    .draft
                    .candidate()
                    .obligations()
                    .iter()
                    .cloned()
                    .map(SourceDiagnostic::Construction),
            );
            SourceFrontier::Construction {
                draft: Box::new(prepared.draft),
                references,
                status: prepared.status.map(Box::new),
            }
        };
        let mut result = SourceCompilation {
            inputs: self.clone(),
            frontier,
            pending,
            diagnostics,
            history,
            identities: ledger,
            effective_audit: None,
            kerml_read_context: Default::default(),
            sysml_read_context: Default::default(),
            lowering_cache: cache.into_inner(),
            work: CompilationWork::default(),
            timings: CompilationTimings::default(),
            edit_frontier: SourceEditFrontier::default(),
            #[cfg(feature = "verification")]
            producer_subjects: observation.finish(),
        };
        // The result already shares unchanged records. Retaining the previous
        // snapshot here would keep obsolete indexes alive without serving the
        // next compilation, which selects its own immediate strict predecessor.
        result.lowering_cache.reconstruction_base = None;
        // Audit the final current graph, including derived local elements that
        // have no direct source-map entry. The accepted dependency is borrowed,
        // never reevaluated as an authored population.
        let audit_started = Instant::now();
        let (audit, capabilities, audit_reused) = effective_audit::run(&result, previous)?;
        timings.borrow_mut().effective_audit_reuse_setup_micros = audit.reuse_setup_micros;
        result.diagnostics.extend(capabilities);
        result.effective_audit = Some(audit);
        timings.borrow_mut().effective_audit_micros = elapsed_micros(audit_started);
        result.work = CompilationWork {
            semantic_cache_used: semantic_cache.is_some(),
            documents_reparsed: self.documents_reparsed,
            documents_lowered: result.lowering_cache.documents_lowered,
            lowering_cache_hits: result.lowering_cache.documents_reused,
            records_lowered: result.lowering_cache.records_lowered,
            records_rebuilt: result.lowering_cache.records_rebuilt,
            records_reused: result.lowering_cache.records_reused,
            producer_subjects_evaluated: result
                .lowering_cache
                .preparatory_producer_subjects_evaluated
                + match &result.frontier {
                    SourceFrontier::Strict(model) => model
                        .producer_status()
                        .map_or(0, |status| status.counters.subjects_evaluated),
                    SourceFrontier::Construction { .. } => 0,
                },
            effective_audit_subjects_evaluated: result
                .effective_audit
                .as_ref()
                .map_or(0, |audit| audit.subjects.len() - audit_reused),
            effective_audit_subjects_reused: audit_reused,
            effective_audit_checks_reused: result
                .effective_audit
                .as_ref()
                .map_or(0, |audit| audit.reused_checks),
        };
        let delta_started = Instant::now();
        result.edit_frontier = SourceEditFrontier::between(previous, &result);
        timings.borrow_mut().edit_frontier_micros = elapsed_micros(delta_started);
        timings.borrow_mut().total_compile_micros = elapsed_micros(compilation_started);
        result.timings = timings.into_inner();
        Ok(result)
    }
    fn lowering_inputs(&self, omitted: &BTreeSet<DocumentId>) -> Vec<SourceInput<'_>> {
        self.documents
            .values()
            .filter(|document| !omitted.contains(&document.id()))
            .map(|document| SourceInput {
                syntax: document
                    .production_syntax()
                    .expect("production source input"),
                library: None,
                sysml: document.language() == SourceLanguage::SysMl,
            })
            .collect()
    }
}

fn prepare_identity_history(
    documents: &BTreeMap<String, Arc<ProjectDocument>>,
    history: &DeclaredConstructionHistory,
    mut ledger: LibrarySourceMap,
) -> Result<(DeclaredConstructionHistory, LibrarySourceMap), agq_kernel::ModelError> {
    let live: BTreeSet<_> = documents
        .values()
        .flat_map(|document| {
            document
                .production_syntax()
                .into_iter()
                .flat_map(|syntax| syntax.nodes().map(|node| (document.id(), node.id())))
        })
        .collect();
    let mut retired = DeclaredIdentitySet::default();
    ledger.retain(|fact, origin| {
        let retained = origin
            .syntax_node
            .is_some_and(|node| live.contains(&(origin.document, node)));
        if !retained {
            match fact {
                FactKey::Element(id) => {
                    retired.elements.insert(*id);
                }
                FactKey::AssociationOccurrence(id) => {
                    retired.occurrences.insert(*id);
                }
                FactKey::Property { .. } => {}
            }
        }
        retained
    });
    Ok((history.retire(&retired)?, ledger))
}

/// Diagnostics retain the native semantic evidence and exact current source origin.
#[derive(Clone, Debug)]
pub enum SourceDiagnostic {
    Syntax {
        document: DocumentId,
        revision: SourceRevisionId,
        status: DocumentStatus,
    },
    Unsupported {
        origin: SourceOrigin,
        construct: String,
    },
    Frontend(FrontendDiagnostic),
    Construction(agq_kernel::ConstructionObligation),
    Capability {
        subject: ElementId,
        origin: Option<SourceOrigin>,
        answer: Box<SysmlQueryResult<Vec<ElementId>>>,
    },
    /// A failed applicable effective operation on this exact authored graph.
    /// Derived subjects can lack a direct source origin; the finding preserves
    /// their canonical identity and native diagnostic instead of inventing one.
    EffectiveAudit {
        origin: Option<SourceOrigin>,
        finding: Box<SystemsPublicationFinding>,
    },
}

/// Immutable authored query audit, constructed only by [`SourceInputs::compile`].
/// This reuses the standard finalizer's effective operations, without conferring
/// standard publication or full language conformance on an authored revision.
#[derive(Debug)]
pub struct SourceEffectiveAudit {
    context: SysmlSemanticContextId,
    subjects: Vec<ElementId>,
    report: SystemsPublicationAudit,
    reuse: Option<effective_audit::AuditReuse>,
    reused_checks: usize,
    reuse_setup_micros: u64,
}
impl SourceEffectiveAudit {
    /// Exact semantic revision and accepted dependencies used by every batch.
    pub fn context(&self) -> &SysmlSemanticContextId {
        &self.context
    }
    /// Every local canonical identity inspected, including derived local records.
    /// Accepted dependency identities are excluded; order is deterministic.
    pub fn subjects(&self) -> &[ElementId] {
        &self.subjects
    }
    /// Applicable query counts and all failures; zero findings alone does not
    /// replace source, reference, construction or producer-certificate gates.
    pub fn report(&self) -> &SystemsPublicationAudit {
        &self.report
    }
}

/// Query factory failure for this exact compilation, never an earlier graph.
#[derive(Clone, Debug)]
pub enum QueryUnavailable {
    Context(String),
}

#[derive(Debug)]
enum SourceFrontier {
    Construction {
        draft: Box<LibraryDraft>,
        references: Vec<ReferenceAssertion>,
        status: Option<Box<AuthoredProducerStatus>>,
    },
    Strict(Box<crate::sysml::SourceModel>),
}

/// Measured work for one authored reconstruction, independent of wall-clock timing.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CompilationWork {
    /// Authenticated persisted effective facts were revalidated on current declarations.
    pub semantic_cache_used: bool,
    /// Actual frontend parse calls while applying this source change batch.
    pub documents_reparsed: usize,
    /// Actual lowering traversals, including repeated full reference-refinement passes.
    pub documents_lowered: usize,
    /// Reused immutable document fragments across all refinement passes.
    pub lowering_cache_hits: usize,
    /// Local declared records traversed by lowering, including rejected trial work.
    pub records_lowered: usize,
    /// Local records created, changed or removed in kernel transactions across
    /// all passes. Every resulting record still undergoes kernel validation.
    pub records_rebuilt: usize,
    /// Unchanged declared records retained from the previous strict snapshot
    /// across construction passes; no record/slot mutations were submitted.
    #[serde(default)]
    pub records_reused: usize,
    /// Subject evaluations across every preparatory and final producer schedule.
    pub producer_subjects_evaluated: usize,
    /// Current local canonical subjects visited by the final effective audit.
    pub effective_audit_subjects_evaluated: usize,
    /// Successful strict audit outcomes retained after explicit read/writer proof.
    #[serde(default)]
    pub effective_audit_subjects_reused: usize,
    /// Actual successful query/family checks transported; zero-query subjects
    /// do not inflate this work-elimination measure.
    #[serde(default)]
    pub effective_audit_checks_reused: usize,
}

/// Observed elapsed times for one authored compilation, in microseconds.
///
/// These measurements are excluded from semantic identities, checkpoints, caches
/// and acceptance. Nested measurements overlap their enclosing phase: source
/// preparation includes construction, refinement and preparatory producers.
/// Parsing happens before compilation and is not included in the total.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CompilationTimings {
    /// Complete authored compile call, excluding input parsing.
    pub total_compile_micros: u64,
    /// Prior identity reconciliation and lowering-cache preparation.
    pub identity_preparation_micros: u64,
    /// Entire declaration/reference preparation, including trial passes.
    pub source_preparation_micros: u64,
    /// Kernel construction and declared identity reconciliation across all passes.
    pub declared_construction_micros: u64,
    /// Refinement context binding, change detection and reference resolution;
    /// excludes construction and producer closure inside construction callbacks.
    pub reference_refinement_micros: u64,
    /// Preparatory producer schedules, checkpoint capture and closure binding.
    pub preparatory_producers_micros: u64,
    /// Final declared graph validation before effective producer closure.
    pub strict_kernel_validation_micros: u64,
    /// Final producer schedule, certificate checkpoint/rebinding and certification.
    /// Certification remains inside the measured closure operation.
    pub final_closure_micros: u64,
    /// Prior producer certificate/context/signature capture inside final closure.
    #[serde(default)]
    pub closure_checkpoint_micros: u64,
    /// Exact prior producer read revalidation inside final closure.
    #[serde(default)]
    pub closure_rebind_micros: u64,
    /// Final source references checked against the closed semantic context.
    pub final_references_micros: u64,
    /// Strict current local-subject audit, including effective context binding.
    pub effective_audit_micros: u64,
    /// Closed-context read signature capture/delta inside the strict audit.
    #[serde(default)]
    pub effective_audit_reuse_setup_micros: u64,
    /// Exact source and declared-fact delta capture after the effective audit.
    pub edit_frontier_micros: u64,
}

pub(crate) fn elapsed_micros(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX)
}

/// Exact authored input/fact delta. Producer invalidation independently checks
/// native query/provider reads, including negative-search populations.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SourceEditFrontier {
    /// Changed documents with old/new source revisions; absent sides mean add/remove.
    pub source_revisions: Vec<(
        DocumentId,
        Option<SourceRevisionId>,
        Option<SourceRevisionId>,
    )>,
    /// New reconciled syntax identities, including replacement nodes.
    pub syntax_nodes_added: BTreeSet<agq_kernel::SyntaxNodeId>,
    /// Removed syntax identities, including nodes replaced by an edit.
    pub syntax_nodes_removed: BTreeSet<agq_kernel::SyntaxNodeId>,
    /// Retained syntax identities whose kind, range, content or ordered children changed.
    pub syntax_nodes_changed: BTreeSet<agq_kernel::SyntaxNodeId>,
    /// New declared canonical facts, excluding producer-generated consequences.
    pub declared_facts_added: BTreeSet<FactKey>,
    /// Removed declared canonical facts.
    pub declared_facts_removed: BTreeSet<FactKey>,
    /// Retained declared fact identities whose values or origins changed.
    pub declared_facts_changed: BTreeSet<FactKey>,
}
impl SourceEditFrontier {
    fn between(previous: Option<&SourceCompilation>, next: &SourceCompilation) -> Self {
        let documents = |compilation: &SourceCompilation| {
            compilation
                .inputs
                .documents()
                .map(|(_, doc)| (doc.id(), doc.revision()))
                .collect::<BTreeMap<_, _>>()
        };
        let before = previous.map(documents).unwrap_or_default();
        let after = documents(next);
        let ids: BTreeSet<_> = before.keys().chain(after.keys()).copied().collect();
        let nodes = |compilation: &SourceCompilation| {
            compilation
                .inputs
                .documents()
                .flat_map(|(_, doc)| {
                    doc.production_syntax()
                        .into_iter()
                        .flat_map(|syntax| syntax.nodes().map(|node| node.id()))
                })
                .collect::<BTreeSet<_>>()
        };
        let old_nodes = previous.map(nodes).unwrap_or_default();
        let new_nodes = nodes(next);
        let mut syntax_nodes_changed = BTreeSet::new();
        if let Some(previous) = previous {
            let old_documents: BTreeMap<_, _> = previous
                .inputs
                .documents()
                .map(|(_, document)| (document.id(), document))
                .collect();
            for (_, document) in next.inputs.documents() {
                if let Some(old) = old_documents.get(&document.id())
                    && !std::ptr::eq(*old, document)
                    && let (Some(before), Some(after)) =
                        (old.production_syntax(), document.production_syntax())
                {
                    syntax_nodes_changed.extend(changed_document_syntax(before, after));
                }
            }
        }
        let before_facts = previous.map(declared_facts).unwrap_or_default();
        let after_facts = declared_facts(next);
        Self {
            source_revisions: ids
                .into_iter()
                .filter_map(|id| {
                    let before = before.get(&id).copied();
                    let after = after.get(&id).copied();
                    (before != after).then_some((id, before, after))
                })
                .collect(),
            syntax_nodes_added: new_nodes.difference(&old_nodes).copied().collect(),
            syntax_nodes_removed: old_nodes.difference(&new_nodes).copied().collect(),
            syntax_nodes_changed,
            declared_facts_added: after_facts
                .keys()
                .filter(|fact| !before_facts.contains_key(fact))
                .copied()
                .collect(),
            declared_facts_removed: before_facts
                .keys()
                .filter(|fact| !after_facts.contains_key(fact))
                .copied()
                .collect(),
            declared_facts_changed: after_facts
                .iter()
                .filter(|(fact, value)| before_facts.get(fact).is_some_and(|old| old != *value))
                .map(|(fact, _)| *fact)
                .collect(),
        }
    }
}

/// Exact retained-node comparison without copying or repeatedly scanning nested
/// source slices. Byte mismatches are indexed once per changed document; each
/// node compares only its own shape and direct ordered child identities.
fn changed_document_syntax(
    before: &syntax::production::Document,
    after: &syntax::production::Document,
) -> BTreeSet<agq_kernel::SyntaxNodeId> {
    let old: BTreeMap<_, _> = before.nodes().map(|node| (node.id(), node)).collect();
    let changed_bytes: Vec<_> = before
        .source()
        .bytes()
        .zip(after.source().bytes())
        .enumerate()
        .filter_map(|(index, (a, b))| (a != b).then_some(index as u64))
        .collect();
    after
        .nodes()
        .filter_map(|node| {
            let prior = old.get(&node.id())?;
            let range = node.range();
            let different = prior.kind() != node.kind()
                || prior.range() != range
                || !prior
                    .children()
                    .map(|child| child.id())
                    .eq(node.children().map(|child| child.id()))
                || changed_bytes
                    .get(changed_bytes.partition_point(|index| *index < range.start()))
                    .is_some_and(|index| *index < range.end());
            different.then_some(node.id())
        })
        .collect()
}

#[cfg(test)]
#[path = "source_frontier_tests.rs"]
mod frontier_tests;
#[derive(PartialEq, Eq)]
enum DeclaredFactValue<'a> {
    Element(agq_kernel::MetaclassId, &'a agq_kernel::provenance::Origin),
    Slot(&'a agq_kernel::Slot),
    Occurrence(&'a agq_kernel::association::AssociationOccurrence),
}
fn declared_facts(compilation: &SourceCompilation) -> BTreeMap<FactKey, DeclaredFactValue<'_>> {
    let model = compilation
        .strict_snapshot()
        .map(Snapshot::model)
        .or_else(|| compilation.construction().map(ConstructionView::model));
    let mut facts = BTreeMap::new();
    if let Some(model) = model {
        let standards = compilation.inputs.accepted_sysml().overlay().model();
        for record in model
            .elements()
            .filter(|record| standards.element(record.id()).is_none())
        {
            facts.insert(
                FactKey::Element(record.id()),
                DeclaredFactValue::Element(record.metaclass(), record.origin()),
            );
            for (property, slot) in record.slots() {
                facts.insert(
                    FactKey::Property {
                        element: record.id(),
                        property,
                    },
                    DeclaredFactValue::Slot(slot),
                );
            }
        }
        for occurrence in model
            .association_occurrences()
            .filter(|occurrence| standards.association_occurrence(occurrence.id()).is_none())
        {
            facts.insert(
                FactKey::AssociationOccurrence(occurrence.id()),
                DeclaredFactValue::Occurrence(occurrence),
            );
        }
    }
    facts
}

/// Immutable declared/derived frontier, certificate, source evidence and checked identity history.
#[derive(Debug)]
pub struct SourceCompilation {
    inputs: Arc<SourceInputs>,
    frontier: SourceFrontier,
    pending: BTreeSet<ElementId>,
    diagnostics: Vec<SourceDiagnostic>,
    history: DeclaredConstructionHistory,
    identities: LibrarySourceMap,
    effective_audit: Option<SourceEffectiveAudit>,
    kerml_read_context:
        std::sync::OnceLock<Result<agq_kerml_semantics::RetainedSemanticContext, QueryUnavailable>>,
    sysml_read_context:
        std::sync::OnceLock<Result<agq_sysml_semantics::RetainedSysmlContext, QueryUnavailable>>,
    lowering_cache: crate::library::construction::LoweringCache,
    work: CompilationWork,
    timings: CompilationTimings,
    edit_frontier: SourceEditFrontier,
    #[cfg(feature = "verification")]
    producer_subjects: BTreeSet<ElementId>,
}
impl SourceCompilation {
    /// Observational phase timings; never a semantic acceptance or cache input.
    pub fn timings(&self) -> &CompilationTimings {
        &self.timings
    }
    /// Measured work, distinct from semantic acceptance or wall-clock latency.
    pub fn work(&self) -> &CompilationWork {
        &self.work
    }
    /// Authored delta; derived consequences remain separate semantic query results.
    pub fn edit_frontier(&self) -> &SourceEditFrontier {
        &self.edit_frontier
    }
    /// Exact source/identity oracle, with document lowering and producer reuse disabled.
    pub fn full_rebuild(&self) -> Result<Self, LibraryLoadError> {
        let mut rebuilt = self.inputs.compile_with_history(
            None,
            Some((self.history.clone(), self.identities.clone())),
            false,
            None,
        )?;
        // The oracle deliberately retains the same parsed source identity inputs.
        rebuilt.work.documents_reparsed = 0;
        Ok(rebuilt)
    }
    pub fn inputs(&self) -> &Arc<SourceInputs> {
        &self.inputs
    }
    pub fn strict_snapshot(&self) -> Option<&Snapshot> {
        match &self.frontier {
            SourceFrontier::Strict(model) => Some(model.snapshot()),
            _ => None,
        }
    }
    pub fn construction(&self) -> Option<&ConstructionView> {
        match &self.frontier {
            SourceFrontier::Construction { draft, .. } => Some(draft.candidate()),
            _ => None,
        }
    }
    pub fn semantic_model(&self) -> Option<&ModelView> {
        Some(match &self.frontier {
            SourceFrontier::Strict(model) => model.semantic_model(),
            SourceFrontier::Construction { draft, .. } => draft.reference_model(),
        })
    }
    pub fn kernel_revision(&self) -> Option<RevisionId> {
        self.strict_snapshot()
            .map(Snapshot::revision)
            .or_else(|| self.construction().map(ConstructionView::revision))
    }
    pub fn diagnostics(&self) -> &[SourceDiagnostic] {
        &self.diagnostics
    }
    /// Effective-operation audit bound to this compilation's immutable context.
    pub fn effective_audit(&self) -> Option<&SourceEffectiveAudit> {
        self.effective_audit.as_ref()
    }
    pub fn references(&self) -> &[ReferenceAssertion] {
        match &self.frontier {
            SourceFrontier::Strict(model) => model.references(),
            SourceFrontier::Construction { references, .. } => references,
        }
    }
    pub fn source_map(&self) -> &LibrarySourceMap {
        match &self.frontier {
            SourceFrontier::Strict(model) => model.source_map(),
            SourceFrontier::Construction { draft, .. } => draft.source_map(),
        }
    }
    pub fn producer_status(&self) -> Option<&AuthoredProducerStatus> {
        match &self.frontier {
            SourceFrontier::Strict(model) => model.producer_status(),
            SourceFrontier::Construction { status, .. } => status.as_deref(),
        }
    }
    pub fn producer_closure(&self) -> Option<&Arc<ProducerClosureCertificate>> {
        match &self.frontier {
            SourceFrontier::Strict(model) => model.producer_closure(),
            SourceFrontier::Construction { draft, .. } => draft.producer_closure(),
        }
    }
    pub fn kerml_queries(&self) -> Result<KerMlQueries<'_>, QueryUnavailable> {
        self.kerml_read_context
            .get_or_init(|| {
                self.fresh_kerml_queries()
                    .map(|queries| queries.retain_context())
            })
            .as_ref()
            .map(|context| KerMlQueries::new(context.borrow()))
            .map_err(Clone::clone)
    }
    fn fresh_kerml_queries(&self) -> Result<KerMlQueries<'_>, QueryUnavailable> {
        match &self.frontier {
            SourceFrontier::Strict(model) => Ok(model.queries()),
            SourceFrontier::Construction { draft, .. } => self
                .inputs
                .dependency
                .candidate_context_with_pending(draft, self.inputs.root, &self.pending)
                .map(KerMlQueries::new)
                .map_err(|error| QueryUnavailable::Context(error.to_string())),
        }
    }
    pub fn sysml_queries(&self) -> Result<SysmlQueries<'_>, QueryUnavailable> {
        self.sysml_read_context
            .get_or_init(|| {
                self.fresh_sysml_queries()
                    .map(|queries| queries.retain_context())
            })
            .as_ref()
            .map(agq_sysml_semantics::RetainedSysmlContext::queries)
            .map_err(Clone::clone)
    }
    fn fresh_sysml_queries(&self) -> Result<SysmlQueries<'_>, QueryUnavailable> {
        match &self.frontier {
            SourceFrontier::Strict(model) => model.sysml_queries().ok_or_else(|| {
                QueryUnavailable::Context("missing authenticated SysML context".into())
            }),
            SourceFrontier::Construction { draft, .. } => {
                let context = self
                    .inputs
                    .dependency
                    .candidate_context_with_pending(draft, self.inputs.root, &self.pending)
                    .map_err(|error| QueryUnavailable::Context(error.to_string()))?;
                let accepted = &self.inputs.dependency.publication;
                SysmlSemanticContext::for_closed_dependency(
                    context,
                    &accepted.identity().dependencies,
                    accepted.bindings().clone(),
                )
                .map(SysmlQueries::new)
                .map_err(|error| QueryUnavailable::Context(format!("{error:?}")))
            }
        }
    }
    /// Actual evaluation events captured at the scheduler boundary, verification only.
    #[cfg(feature = "verification")]
    pub fn observed_producer_subjects(&self) -> impl Iterator<Item = ElementId> + '_ {
        self.producer_subjects.iter().copied()
    }
    /// Inspect the actual retained canonical/index/proof tables, verification only.
    #[cfg(feature = "verification")]
    pub fn dependency_storage(&self) -> agq_kernel::storage_observer::DependencyStorage {
        match &self.frontier {
            SourceFrontier::Strict(model) => model.dependency_storage(),
            SourceFrontier::Construction { draft, .. } => draft.semantic_candidate().map_or_else(
                || agq_kernel::storage_observer::declared_construction_storage(draft.candidate()),
                agq_kernel::storage_observer::construction_storage,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agq_kerml_syntax::production::{self, Production};
    use agq_kernel::provenance::ByteRange;

    #[test]
    fn input_preparation_retains_recovery_without_losing_disjoint_syntax_identity() {
        let source = "package Stable { part def Retained; } package Edited { part def Mutable; }";
        let documents = prepare_documents(
            &BTreeMap::new(),
            [ProjectChange::Add {
                path: "identity.sysml".into(),
                language: SourceLanguage::SysMl,
                source: source.into(),
            }],
            ParseLimits::default(),
            true,
            Some(production::SysmlSyntaxProfile::OperationalV2),
        )
        .unwrap();
        let document = &documents["identity.sysml"];
        let node = |document: &ProjectDocument| {
            document
                .production_syntax()
                .unwrap()
                .nodes()
                .find(|node| {
                    node.kind() == Production::PartDefinition && node.text() == "part def Retained;"
                })
                .unwrap()
                .id()
        };
        let identity = node(document);
        let start = source.find("part def Mutable;").unwrap();
        let recovered = prepare_documents(
            &documents,
            [ProjectChange::Edit {
                document: document.id(),
                edit: TextEdit {
                    range: ByteRange::new(start as u64, source.len() as u64).unwrap(),
                    replacement: "part def Mutable {".into(),
                },
            }],
            ParseLimits::default(),
            true,
            Some(production::SysmlSyntaxProfile::OperationalV2),
        )
        .unwrap();
        assert_eq!(
            recovered["identity.sysml"].status(),
            DocumentStatus::Recovered
        );
        assert_eq!(node(&recovered["identity.sysml"]), identity);
        assert_eq!(documents["identity.sysml"].source(), source);
        let removed = prepare_documents(
            &recovered,
            [ProjectChange::Remove {
                document: document.id(),
            }],
            ParseLimits::default(),
            true,
            Some(production::SysmlSyntaxProfile::OperationalV2),
        )
        .unwrap();
        let added = prepare_documents(
            &removed,
            [ProjectChange::Add {
                path: "identity.sysml".into(),
                language: SourceLanguage::SysMl,
                source: source.into(),
            }],
            ParseLimits::default(),
            true,
            Some(production::SysmlSyntaxProfile::OperationalV2),
        )
        .unwrap();
        assert_ne!(added["identity.sysml"].id(), document.id());
        assert_ne!(node(&added["identity.sysml"]), identity);
    }

    #[test]
    fn unsupported_source_errors_have_current_origin_and_do_not_mask_invariants() {
        let document = production::parse_with_dialect(
            production::Dialect::KerMl,
            DocumentId::new(),
            SourceRevisionId::new(),
            "package P { feature message = \"hello\"; }",
            Default::default(),
        )
        .unwrap();
        assert!(document.is_complete());
        let input = SourceInput {
            syntax: &document,
            library: None,
            sysml: false,
        };
        let base = Snapshot::new(Arc::new(
            agq_kerml::registry_for_profile(agq_kerml::BaselineProfile::OPERATIONAL).unwrap(),
        ));
        let error = crate::library::construction::construct_on(
            &[input],
            &BTreeMap::new(),
            agq_kerml::BaselineProfile::OPERATIONAL,
            base,
            None,
        )
        .unwrap_err();
        let LibraryLoadError::UnsupportedSource { origin, construct } = error else {
            panic!("wrong error classification: {error:?}");
        };
        assert_eq!(origin.document, document.document());
        assert_eq!(origin.revision, document.revision());
        assert!(origin.syntax_node.is_some());
        assert_eq!(construct, "string literal unescaping");

        let overflow = production::parse_with_dialect(
            production::Dialect::KerMl,
            DocumentId::new(),
            SourceRevisionId::new(),
            "package P { feature f = 999999999999999999999999; }",
            Default::default(),
        )
        .unwrap();
        assert!(overflow.is_complete());
        let base = Snapshot::new(Arc::new(
            agq_kerml::registry_for_profile(agq_kerml::BaselineProfile::OPERATIONAL).unwrap(),
        ));
        let large_integer = crate::library::construction::construct_on(
            &[SourceInput {
                syntax: &overflow,
                library: None,
                sysml: false,
            }],
            &BTreeMap::new(),
            agq_kerml::BaselineProfile::OPERATIONAL,
            base,
            None,
        )
        .unwrap();
        let literal = large_integer
            .candidate()
            .model()
            .elements()
            .find(|record| record.metaclass() == agq_kerml::classes::LITERAL_INTEGER)
            .unwrap();
        assert!(large_integer.candidate().model().navigation_slot(literal.id(), agq_kerml::properties::LITERAL_INTEGER_VALUE).unwrap()
            .value().values().any(|value| matches!(value, agq_kernel::value::Value::Integer(value) if value.to_string() == "999999999999999999999999")));

        let safe = production::parse_with_dialect(
            production::Dialect::KerMl,
            DocumentId::new(),
            SourceRevisionId::new(),
            "package P { feature f; }",
            Default::default(),
        )
        .unwrap();
        let inputs = [
            SourceInput {
                syntax: &safe,
                library: None,
                sysml: false,
            },
            SourceInput {
                syntax: &safe,
                library: None,
                sysml: false,
            },
        ];
        let base = Snapshot::new(Arc::new(
            agq_kerml::registry_for_profile(agq_kerml::BaselineProfile::OPERATIONAL).unwrap(),
        ));
        let error = crate::library::construction::construct_on(
            &inputs,
            &BTreeMap::new(),
            agq_kerml::BaselineProfile::OPERATIONAL,
            base,
            None,
        )
        .unwrap_err();
        assert!(
            matches!(error, LibraryLoadError::Interpretation(_)),
            "{error:?}"
        );
    }

    #[test]
    fn deletion_during_recovery_retires_identity_but_temporary_omission_does_not() {
        let source =
            "package Stable { feature retained; } package Edited { feature missing : Missing; }";
        let prepare = |documents: &BTreeMap<String, Arc<ProjectDocument>>,
                       changes: Vec<ProjectChange>| {
            prepare_documents(
                documents,
                changes,
                ParseLimits::default(),
                true,
                Some(production::SysmlSyntaxProfile::OperationalV2),
            )
            .unwrap()
        };
        let documents = prepare(
            &BTreeMap::new(),
            vec![ProjectChange::Add {
                path: "identity.kerml".into(),
                language: SourceLanguage::KerMl,
                source: source.into(),
            }],
        );
        let base = Snapshot::new(Arc::new(
            agq_kerml::registry_for_profile(agq_kerml::BaselineProfile::OPERATIONAL).unwrap(),
        ));
        let construct = |documents: &BTreeMap<String, Arc<ProjectDocument>>| {
            let inputs: Vec<_> = documents
                .values()
                .map(|document| SourceInput {
                    syntax: document.production_syntax().unwrap(),
                    library: None,
                    sysml: false,
                })
                .collect();
            crate::library::construction::construct_on(
                &inputs,
                &BTreeMap::new(),
                agq_kerml::BaselineProfile::OPERATIONAL,
                base.clone(),
                None,
            )
            .unwrap()
        };
        let original = construct(&documents);
        assert!(!original.candidate().obligations().is_empty());
        let ledger: LibrarySourceMap = original
            .source_map()
            .iter()
            .filter(|(fact, _)| {
                matches!(
                    fact,
                    FactKey::Element(_) | FactKey::AssociationOccurrence(_)
                )
            })
            .map(|(fact, origin)| (*fact, origin.clone()))
            .collect();
        let (history, _) = DeclaredConstructionHistory::from_snapshot(&base)
            .reconcile(original.candidate().clone())
            .unwrap();
        let document = &documents["identity.kerml"];
        let node = document
            .production_syntax()
            .unwrap()
            .nodes()
            .find(|node| node.kind() == Production::Feature && node.text() == "feature retained;")
            .unwrap()
            .id();
        let retained: BTreeSet<_> = ledger
            .iter()
            .filter_map(|(fact, origin)| (origin.syntax_node == Some(node)).then_some(*fact))
            .collect();
        assert!(!retained.is_empty());
        let start = source.find("feature missing").unwrap();
        let recovered = prepare(
            &documents,
            vec![ProjectChange::Edit {
                document: document.id(),
                edit: TextEdit {
                    range: ByteRange::new(start as u64, source.len() as u64).unwrap(),
                    replacement: "feature missing {".into(),
                },
            }],
        );
        assert_eq!(
            recovered["identity.kerml"].status(),
            DocumentStatus::Recovered
        );
        let (omitted_history, omitted_ledger) =
            prepare_identity_history(&recovered, &history, ledger).unwrap();
        assert!(
            retained
                .iter()
                .all(|fact| omitted_ledger.contains_key(fact))
        );
        let empty = construct(&BTreeMap::new());
        let (omitted_history, _) = omitted_history
            .reconcile(empty.candidate().clone())
            .unwrap();
        // Returning declarations with surviving syntax identities is legitimate.
        let temporarily_repaired = prepare(
            &recovered,
            vec![ProjectChange::Edit {
                document: document.id(),
                edit: TextEdit {
                    range: ByteRange::new(
                        start as u64,
                        recovered["identity.kerml"].source().len() as u64,
                    )
                    .unwrap(),
                    replacement: "feature missing : Missing; }".into(),
                },
            }],
        );
        let continued = construct(&temporarily_repaired);
        omitted_history
            .reconcile(continued.candidate().clone())
            .unwrap();

        let start = source.find("feature retained;").unwrap();
        let deleted = prepare(
            &recovered,
            vec![ProjectChange::Edit {
                document: document.id(),
                edit: TextEdit {
                    range: ByteRange::new(start as u64, (start + "feature retained;".len()) as u64)
                        .unwrap(),
                    replacement: String::new(),
                },
            }],
        );
        assert_eq!(
            deleted["identity.kerml"].status(),
            DocumentStatus::Recovered
        );
        let (retired_history, retired_ledger) =
            prepare_identity_history(&deleted, &omitted_history, omitted_ledger.clone()).unwrap();
        assert!(
            retained
                .iter()
                .all(|fact| !retired_ledger.contains_key(fact))
        );
        let retired_facts: BTreeSet<_> = omitted_ledger
            .keys()
            .filter(|fact| !retired_ledger.contains_key(fact))
            .copied()
            .collect();
        let error = retired_history
            .reconcile(continued.candidate().clone())
            .unwrap_err();
        assert!(
            matches!(error, agq_kernel::ModelError::ReusedIdentity(id) if retired_facts.contains(&FactKey::Element(id))),
            "{error:?}"
        );
        let readded = prepare(
            &deleted,
            vec![ProjectChange::Edit {
                document: document.id(),
                edit: TextEdit {
                    range: ByteRange::new(start as u64, start as u64).unwrap(),
                    replacement: "feature retained;".into(),
                },
            }],
        );
        let replacement_node = readded["identity.kerml"]
            .production_syntax()
            .unwrap()
            .nodes()
            .find(|node| node.kind() == Production::Feature && node.text() == "feature retained;")
            .unwrap()
            .id();
        assert_ne!(node, replacement_node);
        let repair_start = readded["identity.kerml"]
            .source()
            .find("feature missing")
            .unwrap();
        let repaired = prepare(
            &readded,
            vec![ProjectChange::Edit {
                document: document.id(),
                edit: TextEdit {
                    range: ByteRange::new(
                        repair_start as u64,
                        readded["identity.kerml"].source().len() as u64,
                    )
                    .unwrap(),
                    replacement: "feature missing : Missing; }".into(),
                },
            }],
        );
        assert_eq!(repaired["identity.kerml"].status(), DocumentStatus::Parsed);
        let replacement = construct(&repaired);
        retired_history
            .reconcile(replacement.candidate().clone())
            .unwrap();
        assert!(
            retained
                .iter()
                .all(|fact| !replacement.source_map().contains_key(fact))
        );
    }
}
