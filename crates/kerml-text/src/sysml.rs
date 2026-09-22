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
#[cfg(test)]
#[path = "sysml_tests.rs"]
mod tests;

/// Exact-source construction status, never a Systems Library publication claim.
#[derive(Clone, Debug)]
pub struct SystemsDocumentStatus {
    pub path: String,
    pub document: agq_kernel::DocumentId,
    pub source_sha256: String,
    pub parsed: bool,
    pub byte_exact: bool,
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
    /// Source/graph construction completeness only; SysML producers are separate.
    pub fn construction_complete(&self) -> bool {
        self.documents.iter().all(|document| {
            document.parsed && document.byte_exact && document.construction_gap.is_none()
        }) && self.draft.candidate().obligations().is_empty()
    }
    pub fn queries(&self) -> Result<KerMlQueries<'_>, LibraryLoadError> {
        let pending = if self.construction_complete() {
            BTreeSet::new()
        } else {
            self.draft.roots().iter().copied().collect()
        };
        Ok(KerMlQueries::new(
            self.publication
                .complete_overlay()
                .project_construction_context(
                    self.draft.candidate(),
                    self.draft.roots(),
                    BTreeSet::new(),
                    pending,
                )
                .map_err(|error| LibraryLoadError::Interpretation(format!("{error:?}")))?,
        ))
    }
}

/// Parse the exact pinned 21 Systems documents using the strict shared frontend,
/// retaining individual grammar/construction gaps. No recovery node is lowered.
pub fn prepare_systems_library(
    sources: &VerifiedLibrarySet,
    publication: Arc<CanonicalKermlStandardLibraries>,
) -> Result<SystemsLibraryCandidate, LibraryLoadError> {
    if sources.content_set_id() != publication.source_content_set() {
        return Err(LibraryLoadError::Interpretation(
            "Systems sources and accepted KerML source set differ".into(),
        ));
    }
    let mut parsed = Vec::new();
    let mut documents = Vec::new();
    let base = base(&publication)?;
    for source in sources
        .documents()
        .filter(|source| source.language() == LibraryLanguage::SysMl)
    {
        let syntax = production::parse_sysml(
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
                        base.clone(),
                        None,
                    )
                    .map(|_| ())
                })
                .err()
                .map(|error| error.to_string())
        } else {
            Some(format!(
                "Strict final SysML grammar requires recovery at {:?}",
                syntax.recovery()
            ))
        };
        let supported = construction_gap.is_none();
        documents.push(SystemsDocumentStatus {
            path: source.path().into(),
            document: source.document(),
            source_sha256: source.sha256().into(),
            parsed: syntax.is_complete(),
            byte_exact,
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
    let draft = construction::construct_on(
        &inputs,
        &Default::default(),
        publication.profile(),
        base,
        None,
    )?;
    Ok(SystemsLibraryCandidate {
        draft,
        documents,
        publication,
        source_content_set: sources.content_set_id().into(),
    })
}

#[derive(Debug)]
pub(crate) struct SourceModel {
    snapshot: Snapshot,
    root: ElementId,
    publication: Arc<CanonicalKermlStandardLibraries>,
    references: Vec<ReferenceAssertion>,
    diagnostics: Vec<FrontendDiagnostic>,
    source_map: LibrarySourceMap,
}
impl SourceModel {
    pub(crate) fn snapshot(&self) -> &Snapshot {
        &self.snapshot
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
        KerMlQueries::new(
            self.publication
                .complete_overlay()
                .project_context(&self.snapshot, self.root, BTreeSet::new(), BTreeSet::new())
                .expect("protected immutable publication"),
        )
    }
}

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
    let mut references = Vec::new();
    let mut diagnostics = Vec::new();
    for reference in draft.references() {
        let source = &draft.source_map()[&FactKey::Element(reference.relationship)];
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
            reference_metadata(snapshot.model(), reference.relationship, alias_source)?;
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
                    let actual = snapshot
                        .model()
                        .element(*id)
                        .expect("resolved element")
                        .metaclass();
                    if snapshot
                        .model()
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
        let stored = snapshot
            .model()
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
    drop(q);
    Ok(SourceModel {
        snapshot,
        root,
        publication,
        references,
        diagnostics,
        source_map: draft.source_map().clone(),
    })
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
