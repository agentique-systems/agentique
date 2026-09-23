//! Immutable authored inputs and exact Working compilation; history belongs to callers.
use super::*;
use crate::library::{LibraryDraft, LibraryLoadError, LibrarySourceMap, construction::SourceInput};
use crate::sysml::{
    AcceptedSourceDependency, AuthoredProducerStatus, CanonicalSysmlSystemsLibrary,
};
use agq_kerml_semantics::{Completeness, ProducerClosureCertificate};
use agq_kernel::provenance::{DeclaredOrigin, FactKey, SourceOrigin};
use agq_kernel::{ConstructionView, DeclaredConstructionHistory, DeclaredIdentitySet, ModelView};
use agq_sysml_semantics::{SysmlQueries, SysmlQueryResult, SysmlSemanticContext};
use std::collections::BTreeSet;

/// Exact source inputs. Applying edits shares every unchanged document and syntax arena.
#[derive(Clone, Debug)]
pub struct SourceInputs {
    project: ProjectId,
    root: ElementId,
    limits: ParseLimits,
    dependency: Arc<AcceptedSourceDependency>,
    documents: BTreeMap<String, Arc<ProjectDocument>>,
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
        })
    }
    /// Prepare inputs without modifying this revision. Language recovery is retained.
    pub fn apply(
        &self,
        changes: impl IntoIterator<Item = ProjectChange>,
    ) -> Result<Self, ProjectError> {
        let documents = prepare_documents(
            &self.documents,
            changes,
            self.limits,
            true,
            Some(self.dependency.syntax_profile()),
        )?;
        Ok(Self {
            documents,
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
        let (history, mut ledger) = prepare_identity_history(
            &self.documents,
            &history,
            previous.map_or_else(BTreeMap::new, |previous| previous.identities.clone()),
        )?;
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
            let snapshot = prepared.draft.candidate().clone().revalidate_declared()?;
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
            #[cfg(feature = "verification")]
            producer_subjects: observation.finish(),
        };
        // The capability query preserves unsupported variation and other pending
        // implications even when producer closure itself is Complete.
        let q = result
            .sysml_queries()
            .map_err(|error| LibraryLoadError::Interpretation(format!("{error:?}")))?;
        let mut capabilities = Vec::new();
        for record in q.model().elements().filter(|record| {
            self.dependency
                .publication
                .overlay()
                .model()
                .element(record.id())
                .is_none()
        }) {
            if [agq_sysml::classes::DEFINITION, agq_sysml::classes::USAGE]
                .into_iter()
                .any(|class| {
                    q.model()
                        .registry()
                        .is_subtype(record.metaclass(), class)
                        .unwrap_or(false)
                })
            {
                let answer = q.effective_usages(record.id());
                if answer.completeness() != Completeness::Complete {
                    capabilities.push(SourceDiagnostic::Capability {
                        subject: record.id(),
                        origin: result
                            .source_map()
                            .get(&FactKey::Element(record.id()))
                            .cloned(),
                        answer: Box::new(answer),
                    });
                }
            }
        }
        drop(q);
        result.diagnostics.extend(capabilities);
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

/// Immutable declared/derived frontier, certificate, source evidence and checked identity history.
#[derive(Debug)]
pub struct SourceCompilation {
    inputs: Arc<SourceInputs>,
    frontier: SourceFrontier,
    pending: BTreeSet<ElementId>,
    diagnostics: Vec<SourceDiagnostic>,
    history: DeclaredConstructionHistory,
    identities: LibrarySourceMap,
    #[cfg(feature = "verification")]
    producer_subjects: BTreeSet<ElementId>,
}
impl SourceCompilation {
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
