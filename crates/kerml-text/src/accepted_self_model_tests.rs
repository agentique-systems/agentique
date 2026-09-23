//! Explicit acceptance gate. Missing caches fail when this ignored gate is
//! requested; ordinary unit tests do not replay either standard publication.
use super::*;
use crate::{ProjectChange, ProjectRevision, SourceLanguage, SourceProject};
use agq_sysml_semantics::{
    RequirementCaseRole, StandardSysmlRole, StateSubactionKind, SysmlQueries, SysmlQueryResult,
    SysmlSemanticContext, TransitionFeatureKind, UsageKind,
};
use std::{collections::BTreeMap, fs::File, path::Path};

const CASES: &str = include_str!("../tests/fixtures/agentique-cases.sysml");
const VIEW_METADATA: &str = include_str!("../tests/fixtures/agentique-view-metadata.sysml");

#[test]
fn view_metadata_acceptance_fixture_preserves_real_frontend_classes() {
    let syntax = production::parse_sysml_with_profile(
        production::SysmlSyntaxProfile::OperationalV2,
        DocumentId::from_u128(960_003),
        SourceRevisionId::from_u128(960_004),
        VIEW_METADATA,
        Default::default(),
    )
    .unwrap();
    assert!(syntax.is_complete(), "{:?}", syntax.diagnostics());
    let draft = lower(&syntax);
    let model = draft.candidate().model();
    for (name, class) in [
        ("ArchitectureView", s::VIEW_DEFINITION),
        ("ArchitecturePerspective", s::VIEWPOINT_DEFINITION),
        ("ReviewMetadata", s::METADATA_DEFINITION),
        ("architecture", s::VIEW_USAGE),
        ("perspective", s::VIEWPOINT_USAGE),
    ] {
        assert_eq!(
            model.element(named(model, name)).unwrap().metaclass(),
            class,
            "{name}"
        );
    }
    assert_eq!(
        model.instances(s::METADATA_USAGE, false).unwrap().count(),
        1
    );
}

fn accepted_view_metadata(accepted: &Arc<CanonicalSysmlSystemsLibrary>) {
    let mut project =
        SourceProject::with_accepted_sysml_standard_libraries(accepted.clone()).unwrap();
    let revision = project
        .apply(
            project.current().revision(),
            [ProjectChange::Add {
                path: "PresentationAcceptance.sysml".into(),
                language: SourceLanguage::SysMl,
                source: VIEW_METADATA.into(),
            }],
        )
        .unwrap();
    let status = revision.producer_status().unwrap();
    assert!(status.converged, "{status:?}");
    assert_eq!(status.completeness, Completeness::Complete, "{status:?}");
    assert!(
        revision.is_complete_slice(),
        "{:?}",
        revision.semantic_diagnostics()
    );
    assert!(
        revision
            .producer_closure()
            .unwrap()
            .is_fully_closed(revision.semantic_model())
    );
    for reference in revision.references() {
        assert_eq!(reference.resolution.completeness, Completeness::Complete);
        assert!(matches!(
            reference.resolution.value,
            Resolution::Resolved(_)
        ));
    }
    let q = revision.sysml_queries().unwrap();
    let inherited = q.effective_usages(authored_named(q.model(), "SpecializedPresentation"));
    complete(&inherited);
    let metadata = q
        .model()
        .elements()
        .find(|record| {
            record.metaclass() == s::METADATA_USAGE
                && matches!(
                    record.origin(),
                    Origin::Declared(DeclaredOrigin::Authored { .. })
                )
        })
        .expect("authored prefix MetadataUsage")
        .id();
    for (definition, usage, role) in [
        (
            "ArchitectureView",
            authored_named(q.model(), "architecture"),
            StandardSysmlRole::View,
        ),
        ("ReviewMetadata", metadata, StandardSysmlRole::MetadataItem),
        (
            "ArchitecturePerspective",
            authored_named(q.model(), "perspective"),
            StandardSysmlRole::ViewpointCheck,
        ),
    ] {
        let definition = authored_named(q.model(), definition);
        let parents = q.effective_supertypes(definition);
        complete(&parents);
        assert!(
            parents
                .value()
                .contains(&accepted.bindings().targets()[&role])
        );
        let types = q.effective_usage_types(usage);
        complete(&types);
        assert!(types.value().contains(&definition));
    }
    assert!(
        inherited
            .value()
            .contains(&authored_named(q.model(), "architecture")),
        "inherited ViewUsage retains its original identity"
    );
    assert!(
        inherited
            .value()
            .contains(&authored_named(q.model(), "perspective")),
        "inherited ViewpointUsage retains its original identity"
    );
}

#[test]
fn case_acceptance_fixture_uses_real_frontend_and_explicit_standard_redefinitions() {
    let syntax = production::parse_sysml_with_profile(
        production::SysmlSyntaxProfile::OperationalV2,
        DocumentId::from_u128(960_001),
        SourceRevisionId::from_u128(960_002),
        CASES,
        Default::default(),
    )
    .unwrap();
    assert!(syntax.is_complete(), "{:?}", syntax.diagnostics());
    let draft = lower(&syntax);
    // Only the accepted-cache gate supplies these standard namespaces. This
    // frontend test deliberately retains the unresolved standard endpoints.
    assert!(!draft.candidate().obligations().is_empty());
    let model = draft.candidate().model();
    for (name, class) in [
        ("ReviewCase", s::CASE_DEFINITION),
        ("VerifyRevision", s::VERIFICATION_CASE_DEFINITION),
        ("plannedReview", s::CASE_USAGE),
        ("plannedVerification", s::VERIFICATION_CASE_USAGE),
        ("reviewer", s::PART_USAGE),
        ("verifier", s::PART_USAGE),
        ("preservation", s::REQUIREMENT_USAGE),
        ("verificationObjective", s::REQUIREMENT_USAGE),
    ] {
        assert_eq!(
            model.element(named(model, name)).unwrap().metaclass(),
            class
        );
    }
    let query = KerMlQueries::new(
        SemanticContext::for_construction(draft.candidate(), options(), BTreeSet::new()).unwrap(),
    );
    for (name, class) in [
        ("reviewWorkspace", s::SUBJECT_MEMBERSHIP),
        ("verifiedWorkspace", s::SUBJECT_MEMBERSHIP),
        ("reviewer", s::ACTOR_MEMBERSHIP),
        ("verifier", s::ACTOR_MEMBERSHIP),
        ("preservation", s::OBJECTIVE_MEMBERSHIP),
        ("verificationObjective", s::OBJECTIVE_MEMBERSHIP),
        ("reviewResult", c::RETURN_PARAMETER_MEMBERSHIP),
        ("verdict", c::RETURN_PARAMETER_MEMBERSHIP),
    ] {
        let membership = query.owning_relationship(named(model, name)).value.unwrap();
        assert_eq!(
            model.element(membership).unwrap().metaclass(),
            class,
            "{name}"
        );
    }
}

fn complete<T: std::fmt::Debug>(answer: &SysmlQueryResult<T>) {
    assert_eq!(answer.completeness(), Completeness::Complete, "{answer:?}");
}

fn authored_named(model: &ModelView, name: &str) -> ElementId {
    let ids: Vec<_> = model
        .elements()
        .filter(|record| {
            matches!(
                record.origin(),
                Origin::Declared(DeclaredOrigin::Authored { .. })
            ) && model
                .navigation_slot(record.id(), p::ELEMENT_DECLARED_NAME)
                .is_some_and(|slot| {
                    slot.value()
                        .values()
                        .any(|value| value == &Value::String(name.into()))
                })
        })
        .map(|record| record.id())
        .collect();
    assert_eq!(ids.len(), 1, "authored declaration {name}");
    ids[0]
}

fn names(queries: &SysmlQueries<'_>, ids: impl IntoIterator<Item = ElementId>) -> BTreeSet<String> {
    ids.into_iter()
        .filter_map(|id| {
            queries
                .model()
                .navigation_slot(id, p::ELEMENT_DECLARED_NAME)
                .and_then(|slot| match slot.value() {
                    SlotValue::Scalar(Value::String(name)) => Some(name.clone()),
                    _ => None,
                })
        })
        .collect()
}

/// Compare local semantic roles across independent ID allocation, while standard
/// targets retain their exact shared identity. Names alone are not identities:
/// unnamed children and repeated names remain visible through ownership/order.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum ComparisonElement {
    Dependency(ElementId),
    Local {
        class: agq_kernel::MetaclassId,
        name: Option<String>,
        owner: Option<Box<ComparisonElement>>,
        carrier: Option<agq_kernel::MetaclassId>,
        ordinal: usize,
    },
}

#[derive(Clone, Copy)]
enum ComparisonOrder {
    Unordered,
    Ordered,
}

type RichSummary = BTreeMap<String, Vec<ComparisonElement>>;

fn declared_name(model: &ModelView, id: ElementId) -> Option<String> {
    model
        .navigation_slot(id, p::ELEMENT_DECLARED_NAME)
        .and_then(|slot| match slot.value() {
            SlotValue::Scalar(Value::String(name)) => Some(name.clone()),
            _ => None,
        })
}

fn comparison_element(
    q: &KerMlQueries<'_>,
    dependency: &ModelView,
    id: ElementId,
    visiting: &mut BTreeSet<ElementId>,
) -> ComparisonElement {
    if dependency.element(id).is_some() {
        return ComparisonElement::Dependency(id);
    }
    assert!(visiting.insert(id), "acyclic local ownership for {id}");
    let model = q.model();
    let class = model.element(id).unwrap().metaclass();
    let name = declared_name(model, id);
    let owning = q.owning_relationship(id);
    assert_eq!(owning.completeness, Completeness::Complete, "{owning:?}");
    let (owner, carrier, ordinal) = if let Some(membership) = owning.value {
        let carrier = model.element(membership).unwrap().metaclass();
        let owner = q.owning_related_element(membership);
        assert_eq!(owner.completeness, Completeness::Complete, "{owner:?}");
        let owner = owner.value.expect("owned member has a canonical owner");
        let relationships = q.owned_relationships(owner);
        assert_eq!(
            relationships.completeness,
            Completeness::Complete,
            "{relationships:?}"
        );
        let siblings: Vec<_> = relationships
            .value
            .into_iter()
            .filter(|&relationship| model.element(relationship).unwrap().metaclass() == carrier)
            .flat_map(|relationship| {
                model
                    .navigation_slot(relationship, p::RELATIONSHIP_OWNED_RELATED_ELEMENT)
                    .into_iter()
                    .flat_map(|slot| slot.value().values())
                    .filter_map(|value| match value {
                        Value::Reference(element) => Some(*element),
                        _ => None,
                    })
            })
            .filter(|&sibling| {
                model.element(sibling).unwrap().metaclass() == class
                    && declared_name(model, sibling) == name
            })
            .collect();
        assert_eq!(siblings.iter().filter(|&&sibling| sibling == id).count(), 1);
        let ordinal = siblings.iter().position(|&sibling| sibling == id).unwrap();
        (
            Some(Box::new(comparison_element(q, dependency, owner, visiting))),
            Some(carrier),
            ordinal,
        )
    } else {
        // The fixture has one unnamed authored namespace root. Reject an
        // ambiguous root role instead of normalizing two distinct roots together.
        let peers = model
            .elements()
            .filter(|record| {
                dependency.element(record.id()).is_none()
                    && record.metaclass() == class
                    && declared_name(model, record.id()) == name
            })
            .filter(|record| {
                let owner = q.owning_relationship(record.id());
                assert_eq!(owner.completeness, Completeness::Complete, "{owner:?}");
                owner.value.is_none()
            })
            .count();
        assert_eq!(peers, 1, "unique local root role for {id}");
        (None, None, 0)
    };
    visiting.remove(&id);
    ComparisonElement::Local {
        class,
        name,
        owner,
        carrier,
        ordinal,
    }
}

fn comparison_population(
    q: &KerMlQueries<'_>,
    dependency: &ModelView,
    ids: &[ElementId],
    order: ComparisonOrder,
) -> Vec<ComparisonElement> {
    let mut population: Vec<_> = ids
        .iter()
        .map(|&id| comparison_element(q, dependency, id, &mut BTreeSet::new()))
        .collect();
    if matches!(order, ComparisonOrder::Unordered) {
        population.sort(); // Preserve multiplicity; never filter or deduplicate.
    }
    population
}

#[test]
fn comparison_preserves_unnamed_members_multiplicity_dependency_ids_and_order() {
    let snapshot = |source: &str| lower(&parse(source)).strict_snapshot().unwrap();
    let original = snapshot("package Comparison { part def Owner { part same; part; } }");
    let independent = snapshot("package Comparison { part def Owner { part same; part; } }");
    let extra_unnamed =
        snapshot("package Comparison { part def Owner { part same; part; part; } }");
    let repeated_name =
        snapshot("package Comparison { part def Owner { part same; part same; part; } }");
    let empty = Snapshot::new(Arc::new(
        agq_sysml::registry_for_profile(BaselineProfile::OPERATIONAL_V9).unwrap(),
    ));
    let population = |snapshot: &Snapshot| {
        let queries = q(snapshot);
        let children = queries.direct_features(named(snapshot.model(), "Owner"));
        assert_eq!(children.completeness, Completeness::Complete);
        comparison_population(
            &queries,
            empty.model(),
            &children.value,
            ComparisonOrder::Unordered,
        )
    };
    assert_ne!(
        named(original.model(), "Owner"),
        named(independent.model(), "Owner"),
        "independent frontend allocations"
    );
    assert_eq!(population(&original), population(&independent));
    assert_eq!(population(&original).len(), 2);
    assert_ne!(population(&original), population(&extra_unnamed));
    assert_ne!(population(&original), population(&repeated_name));

    let queries = q(&original);
    let children = queries.direct_features(named(original.model(), "Owner"));
    assert_eq!(children.completeness, Completeness::Complete);
    let ordered = comparison_population(
        &queries,
        empty.model(),
        &children.value,
        ComparisonOrder::Ordered,
    );
    let reversed: Vec<_> = children.value.iter().copied().rev().collect();
    assert_ne!(
        ordered,
        comparison_population(&queries, empty.model(), &reversed, ComparisonOrder::Ordered)
    );
    assert_eq!(
        population(&original),
        comparison_population(
            &queries,
            empty.model(),
            &reversed,
            ComparisonOrder::Unordered
        )
    );
    let duplicated = comparison_population(
        &queries,
        empty.model(),
        &[children.value[0], children.value[0]],
        ComparisonOrder::Unordered,
    );
    assert_eq!(duplicated.len(), 2);
    assert_eq!(duplicated[0], duplicated[1]);

    let queries = q(&repeated_name);
    let children = queries.direct_features(named(repeated_name.model(), "Owner"));
    assert_eq!(children.completeness, Completeness::Complete);
    let same: Vec<_> = children
        .value
        .into_iter()
        .filter(|&id| declared_name(repeated_name.model(), id).as_deref() == Some("same"))
        .collect();
    assert_eq!(same.len(), 2);
    let locals = comparison_population(&queries, empty.model(), &same, ComparisonOrder::Unordered);
    assert_ne!(locals[0], locals[1], "same-name canonical siblings differ");
    // This neutral graph checks key selection only; it is not an accepted
    // publication. The real gate supplies its authenticated Systems dependency.
    let dependencies = comparison_population(
        &queries,
        repeated_name.model(),
        &same,
        ComparisonOrder::Unordered,
    );
    assert_ne!(dependencies[0], dependencies[1]);
    assert!(
        dependencies
            .iter()
            .all(|key| matches!(key, ComparisonElement::Dependency(_)))
    );
}

fn assert_revision(revision: &ProjectRevision, accepted: &CanonicalSysmlSystemsLibrary) {
    let status = revision
        .producer_status()
        .expect("accepted authored scheduler");
    assert!(status.converged, "{status:?}");
    assert_eq!(status.completeness, Completeness::Complete, "{status:?}");
    assert!(
        revision.is_complete_slice(),
        "{:?}",
        revision.semantic_diagnostics()
    );
    assert!(revision.semantic_diagnostics().is_empty());
    assert!(
        revision
            .producer_closure()
            .expect("accepted authored closure certificate")
            .is_fully_closed(revision.semantic_model())
    );
    assert!(std::ptr::eq(
        revision.snapshot().immutable_dependency().unwrap().as_ref(),
        accepted.overlay()
    ));
    assert_eq!(
        status.counters.declared_subjects,
        revision
            .snapshot()
            .model()
            .elements()
            .filter(|record| !revision.snapshot().is_dependency_element(record.id()))
            .count()
    );
    for reference in revision.references() {
        assert_eq!(
            reference.resolution.completeness,
            Completeness::Complete,
            "{reference:?}"
        );
        assert!(
            matches!(reference.resolution.value, Resolution::Resolved(_)),
            "{reference:?}"
        );
    }
    let q = revision.sysml_queries().unwrap();
    for (definition, role) in [
        ("ProjectWorkspace", StandardSysmlRole::Part),
        ("SemanticState", StandardSysmlRole::Item),
        ("SemanticQuery", StandardSysmlRole::Port),
        ("ValidateRevision", StandardSysmlRole::Action),
        ("Serving", StandardSysmlRole::StateAction),
    ] {
        let parents = q.effective_supertypes(authored_named(q.model(), definition));
        complete(&parents);
        assert!(
            parents
                .value()
                .contains(&accepted.bindings().targets()[&role]),
            "{definition}: {parents:?}"
        );
    }
}

fn architecture_invariants(q: &SysmlQueries<'_>) {
    let model = q.model();
    let deps = |owner: &str| {
        let members = q.owned_usages(authored_named(model, owner));
        complete(&members);
        let mut targets = BTreeSet::new();
        for &member in members.value() {
            if model.element(member).unwrap().metaclass() == s::PART_USAGE
                && model
                    .navigation_slot(member, p::FEATURE_IS_COMPOSITE)
                    .is_some_and(|slot| slot.value() == &SlotValue::Scalar(Value::Boolean(false)))
            {
                let types = q.kerml().direct_feature_types(member);
                assert_eq!(types.completeness, Completeness::Complete);
                targets.extend(names(q, types.value));
            }
        }
        targets
    };
    assert!(deps("SemanticKernel").is_empty());
    assert_eq!(
        deps("KerMLEngine"),
        BTreeSet::from(["SemanticKernel".into()])
    );
    assert_eq!(deps("SysMLEngine"), BTreeSet::from(["KerMLEngine".into()]));
    assert_eq!(
        deps("ProjectWorkspace"),
        BTreeSet::from(["StandardLibraryManager".into(), "SysMLEngine".into()])
    );
    assert_eq!(deps("ViewService"), BTreeSet::from(["QueryService".into()]));
    assert_eq!(
        deps("ExecutionCompiler"),
        BTreeSet::from(["ValidationService".into()])
    );
    for (owner, declarations) in [
        (
            "Agentique",
            &[
                ("languageSubsystem", "LanguageEngine"),
                ("modeling", "ModelingPlatform"),
                ("execution", "ExecutionSubsystem"),
            ][..],
        ),
        (
            "LanguageEngine",
            &[
                ("semanticKernel", "SemanticKernel"),
                ("kermlEngine", "KerMLEngine"),
                ("sysmlEngine", "SysMLEngine"),
                ("standardLibraries", "StandardLibraryManager"),
            ][..],
        ),
        (
            "ExecutionSubsystem",
            &[
                ("compiler", "ExecutionCompiler"),
                ("executionIR", "ExecutionIR"),
                ("simulation", "SimulationRuntime"),
            ][..],
        ),
    ] {
        let parts = q.owned_usages_of_kind(authored_named(model, owner), UsageKind::Part);
        complete(&parts);
        assert_eq!(
            names(q, parts.value().iter().copied()),
            declarations
                .iter()
                .map(|(name, _)| (*name).into())
                .collect::<BTreeSet<_>>(),
            "{owner} composition"
        );
        for (name, definition) in declarations {
            let part = authored_named(model, name);
            assert!(parts.value().contains(&part), "{owner}::{name}");
            let types = q.effective_part_definitions(part);
            complete(&types);
            assert!(
                types.value().contains(&authored_named(model, definition)),
                "{owner}::{name}: {types:?}"
            );
        }
    }
    let workspace = q.effective_usages(authored_named(model, "ProjectWorkspace"));
    complete(&workspace);
    for declaration in ["kermlPublication", "systemsPublication", "languageQueries"] {
        assert!(
            workspace
                .value()
                .contains(&authored_named(model, declaration))
        );
    }
    let input = q.effective_item_definitions(authored_named(model, "validatedInput"));
    complete(&input);
    assert!(
        input
            .value()
            .contains(&authored_named(model, "ValidatedSemanticState"))
    );
    assert_eq!(
        deps("SimulationRuntime"),
        BTreeSet::from(["ExecutionIR".into()])
    );
}

fn rich_summary(q: &SysmlQueries<'_>, dependency: &ModelView) -> RichSummary {
    let mut summary = BTreeMap::new();
    for owner in [
        "ProjectWorkspace",
        "IncrementalWorkspace",
        "ModelingPlatform",
        "ValidateRevision",
        "Serving",
        "ImmutableRevisions",
    ] {
        let usages = q.effective_usages(authored_named(q.model(), owner));
        complete(&usages);
        summary.insert(
            format!("usages/{owner}"),
            comparison_population(
                q.kerml(),
                dependency,
                usages.value(),
                ComparisonOrder::Unordered,
            ),
        );
    }
    for (label, answer) in [
        (
            "ports",
            q.effective_ports(authored_named(q.model(), "IncrementalWorkspace")),
        ),
        (
            "nested",
            q.effective_nested_usages(authored_named(q.model(), "workspaceQuery")),
        ),
        (
            "subparts",
            q.effective_subparts(authored_named(q.model(), "ModelingPlatform")),
        ),
        (
            "subitems",
            q.effective_subitems(authored_named(q.model(), "ModelingPlatform")),
        ),
        (
            "attribute",
            q.effective_attribute_definitions(authored_named(q.model(), "acceptedRevision")),
        ),
        (
            "part",
            q.effective_part_definitions(authored_named(q.model(), "workspace")),
        ),
        (
            "item",
            q.effective_item_definitions(authored_named(q.model(), "checkedState")),
        ),
        (
            "usage-types",
            q.effective_usage_types(authored_named(q.model(), "workspace")),
        ),
        (
            "port-definition",
            q.effective_port_definitions(authored_named(q.model(), "workspaceQuery")),
        ),
        (
            "subsetting",
            q.effective_subsetted_features(authored_named(q.model(), "acceptedRevision")),
        ),
        (
            "redefinition",
            q.effective_redefined_features(authored_named(q.model(), "acceptedRevision")),
        ),
        (
            "interface",
            q.effective_interface_ends(authored_named(q.model(), "SemanticAccess")),
        ),
        (
            "ends",
            q.effective_connection_related_features(authored_named(q.model(), "queryConnection")),
        ),
        (
            "parameters",
            q.effective_parameters(authored_named(q.model(), "ValidateRevision")),
        ),
        (
            "subactions",
            q.effective_subactions(authored_named(q.model(), "ModelingPlatform")),
        ),
        (
            "occurrences",
            q.effective_usages_of_kind(
                authored_named(q.model(), "ModelingPlatform"),
                UsageKind::Occurrence,
            ),
        ),
        (
            "constraints",
            q.effective_usages_of_kind(
                authored_named(q.model(), "ImmutableRevisions"),
                UsageKind::Constraint,
            ),
        ),
        (
            "entry",
            q.state_actions(
                authored_named(q.model(), "Serving"),
                StateSubactionKind::Entry,
            ),
        ),
        (
            "do",
            q.state_actions(authored_named(q.model(), "Serving"), StateSubactionKind::Do),
        ),
        (
            "exit",
            q.state_actions(
                authored_named(q.model(), "Serving"),
                StateSubactionKind::Exit,
            ),
        ),
        (
            "subject",
            q.requirement_case_features(
                authored_named(q.model(), "ImmutableRevisions"),
                RequirementCaseRole::Subject,
            ),
        ),
    ] {
        complete(&answer);
        let expected: &[&str] = match label {
            "ports" => &["workspaceQuery"],
            "nested" => &["semanticAnswer"],
            "subparts" | "subitems" => &["workspace"],
            "attribute" => &["RevisionNumber"],
            "part" | "usage-types" => &["ProjectWorkspace"],
            "item" => &["ValidatedSemanticState"],
            "port-definition" => &["SemanticQuery"],
            "subsetting" | "redefinition" => &["revisionNumber"],
            "interface" => &["requester", "responder"],
            "parameters" => &["workingState", "checkedState"],
            "subactions" => &["validate"],
            "occurrences" => &["workspace", "validate", "operating"],
            "constraints" => &["preservesPriorState"],
            "entry" => &["startServing"],
            "do" => &["checkRevision"],
            "exit" => &["stopServing"],
            "subject" => &["subjectWorkspace"],
            _ => &[],
        };
        for &declaration in expected {
            assert!(
                answer
                    .value()
                    .contains(&authored_named(q.model(), declaration)),
                "{label} must retain the canonical {declaration}: {answer:?}"
            );
        }
        let order = match label {
            "parameters" | "ends" | "interface" | "entry" | "do" | "exit" => {
                ComparisonOrder::Ordered
            }
            _ => ComparisonOrder::Unordered,
        };
        summary.insert(
            label.into(),
            comparison_population(q.kerml(), dependency, answer.value(), order),
        );
    }
    let inherited = q.effective_usages(authored_named(q.model(), "IncrementalWorkspace"));
    assert_eq!(
        inherited
            .value()
            .iter()
            .filter(|&&id| names(q, [id]).contains("workspaceQuery"))
            .count(),
        1,
        "inherited lookup retains the one canonical member"
    );
    assert!(
        inherited
            .value()
            .contains(&authored_named(q.model(), "workspaceQuery"))
    );
    assert!(
        inherited
            .value()
            .contains(&authored_named(q.model(), "acceptedRevision"))
    );
    assert!(
        !inherited
            .value()
            .contains(&authored_named(q.model(), "revisionNumber"))
    );
    let related =
        q.effective_connection_related_features(authored_named(q.model(), "queryConnection"));
    complete(&related);
    assert_eq!(
        related.value(),
        &vec![
            authored_named(q.model(), "clientQueries"),
            authored_named(q.model(), "platformQueries")
        ],
        "authored endpoint order"
    );
    let connection = authored_named(q.model(), "queryConnection");
    let ends = q.effective_connection_ends(connection);
    let interface_ends = q.effective_interface_ends(connection);
    complete(&ends);
    complete(&interface_ends);
    assert_eq!(ends.value(), interface_ends.value());
    assert_eq!(ends.value().len(), 2);
    for (&end, &endpoint) in ends.value().iter().zip(related.value()) {
        assert_eq!(q.model().element(end).unwrap().metaclass(), s::PORT_USAGE);
        let references = q
            .kerml()
            .owned_relationships_of_type(end, c::REFERENCE_SUBSETTING);
        assert_eq!(references.completeness, Completeness::Complete);
        assert_eq!(references.value.len(), 1);
        assert_eq!(
            q.model()
                .navigation_slot(
                    references.value[0],
                    p::REFERENCE_SUBSETTING_REFERENCED_FEATURE
                )
                .unwrap()
                .value(),
            &SlotValue::Scalar(Value::Reference(endpoint)),
            "each anonymous end retains its own canonical endpoint in order"
        );
    }
    let path = q.effective_qualified_name(authored_named(q.model(), "acceptedRevision"));
    complete(&path);
    let segments = &path.value().as_ref().unwrap().segments;
    assert_eq!(
        segments,
        &vec![
            BTreeSet::from(["PlatformArchitecture".into()]),
            BTreeSet::from(["IncrementalWorkspace".into()]),
            BTreeSet::from(["acceptedRevision".into()])
        ]
    );
    summary
}

fn accepted_trigger_and_message(
    revision: &ProjectRevision,
    accepted: &CanonicalSysmlSystemsLibrary,
) {
    let q = revision.sysml_queries().unwrap();
    let lookup = |scope, segments: &[&str]| {
        let answer = q.kerml().lookup_path(
            scope,
            &agq_kerml_semantics::QualifiedName {
                absolute: false,
                segments: segments.iter().map(|segment| (*segment).into()).collect(),
            },
        );
        assert_eq!(answer.completeness, Completeness::Complete, "{answer:?}");
        assert_eq!(answer.value.len(), 1, "{answer:?}");
        answer.value[0].element
    };
    // Pinned Actions::AcceptAction owns the transition's unnamed accepter.
    // The library's acceptedMessage feature is separate from payloadParameter.
    let transition = lookup(
        revision.root(),
        &["Actions", "AcceptAction", "aState", "aTransition"],
    );
    let triggers = q.transition_features(transition, TransitionFeatureKind::Trigger);
    complete(&triggers);
    assert_eq!(triggers.value().len(), 1);
    let accepter = triggers.value()[0];
    assert_eq!(
        q.model().element(accepter).unwrap().metaclass(),
        s::ACCEPT_ACTION_USAGE
    );
    let ancestors = q.kerml().all_supertypes(accepter);
    assert_eq!(ancestors.completeness, Completeness::Complete);
    assert!(
        ancestors
            .value
            .contains(&accepted.bindings().targets()[&StandardSysmlRole::TransitionAccepter])
    );
    let payload = q.accept_action_payload_parameter(accepter);
    complete(&payload);
    assert_eq!(payload.value().len(), 1);
    let message = lookup(accepter, &["acceptedMessage"]);
    assert_ne!(payload.value()[0], message);
    let members = q.effective_usages(accepter);
    complete(&members);
    assert!(members.value().contains(&message));
    assert!(members.value().contains(&payload.value()[0]));
}

struct ProgrammaticSemantics {
    summary: RichSummary,
    workspace_query: ElementId,
    dependency: Arc<agq_kernel::derived::DerivedOverlay>,
}

fn programmatic_semantics(accepted: &Arc<CanonicalSysmlSystemsLibrary>) -> ProgrammaticSemantics {
    let witness = accepted.producer_closed_dependency().unwrap();
    let snapshot =
        super::programmatic_vertical::programmatic_platform_on(witness.project_snapshot());
    // The independent builder allocates this unnamed root before any other element.
    let root = ElementId::from_u128(910_000);
    let roots = std::iter::once(root)
        .chain(accepted.roots().iter().copied())
        .chain(accepted.accepted_kerml().roots().iter().copied())
        .collect();
    let extension = agq_sysml_semantics::SysmlProducerExtension::new(
        accepted.identity().dependencies.sysml_profile,
        accepted.bindings().clone(),
        roots,
    );
    let closed = agq_kerml_semantics::close_result_structure_with_extension(
        &snapshot,
        Default::default(),
        |overlay| {
            witness
                .project_overlay_context(overlay, &[root])
                .map_err(agq_kerml_semantics::PublicationOverlayError::Context)
        },
        &extension,
        |_, _, _, _| {},
        |_| {},
    )
    .unwrap();
    assert!(closed.converged, "{:?}", closed.stages);
    assert_eq!(
        closed.completeness,
        Completeness::Complete,
        "{:?}",
        closed.stages
    );
    assert!(
        closed
            .certificate
            .as_ref()
            .expect("programmatic closure certificate")
            .is_fully_closed(closed.overlay.model())
    );
    let context = witness
        .project_overlay_context(&closed.overlay, &[root])
        .unwrap()
        .with_producer_closure(closed.certificate.unwrap())
        .unwrap();
    let q = SysmlQueries::new(
        SysmlSemanticContext::for_closed_dependency(
            context,
            &accepted.identity().dependencies,
            accepted.bindings().clone(),
        )
        .unwrap(),
    );
    // Retain comparison observations and the already-shared publication handle,
    // not the independent model's complete declared/derived maps and indexes.
    ProgrammaticSemantics {
        summary: rich_summary(&q, accepted.overlay().model()),
        workspace_query: authored_named(q.model(), "workspaceQuery"),
        dependency: snapshot.immutable_dependency().unwrap().clone(),
    }
}

fn accepted_case_roles(accepted: &Arc<CanonicalSysmlSystemsLibrary>) {
    let mut project =
        SourceProject::with_accepted_sysml_standard_libraries(accepted.clone()).unwrap();
    let revision = project
        .apply(
            project.current().revision(),
            [ProjectChange::Add {
                path: "WorkspaceAcceptance.sysml".into(),
                language: SourceLanguage::SysMl,
                source: CASES.into(),
            }],
        )
        .unwrap();
    let status = revision.producer_status().unwrap();
    assert!(status.converged, "{status:?}");
    assert_eq!(status.completeness, Completeness::Complete, "{status:?}");
    assert!(
        revision.is_complete_slice(),
        "{:?}",
        revision.semantic_diagnostics()
    );
    assert!(
        revision
            .producer_closure()
            .expect("case-role closure certificate")
            .is_fully_closed(revision.semantic_model())
    );
    for reference in revision.references() {
        assert_eq!(
            reference.resolution.completeness,
            Completeness::Complete,
            "{reference:?}"
        );
        assert!(
            matches!(reference.resolution.value, Resolution::Resolved(_)),
            "{reference:?}"
        );
    }
    let q = revision.sysml_queries().unwrap();
    for (definition, usage, role, subject, actor, objective, result) in [
        (
            "ReviewCase",
            "plannedReview",
            StandardSysmlRole::Case,
            "reviewWorkspace",
            "reviewer",
            "preservation",
            "reviewResult",
        ),
        (
            "VerifyRevision",
            "plannedVerification",
            StandardSysmlRole::VerificationCase,
            "verifiedWorkspace",
            "verifier",
            "verificationObjective",
            "verdict",
        ),
    ] {
        let parents = q.effective_supertypes(authored_named(q.model(), definition));
        complete(&parents);
        assert!(
            parents
                .value()
                .contains(&accepted.bindings().targets()[&role])
        );
        for owner in [definition, usage] {
            for (member_role, member) in [
                (RequirementCaseRole::Subject, subject),
                (RequirementCaseRole::Actor, actor),
                (RequirementCaseRole::Objective, objective),
            ] {
                let answer =
                    q.requirement_case_features(authored_named(q.model(), owner), member_role);
                complete(&answer);
                assert_eq!(
                    answer.value(),
                    &[authored_named(q.model(), member)],
                    "{owner} {member_role:?}"
                );
            }
            let answer = q.effective_return_parameters(authored_named(q.model(), owner));
            complete(&answer);
            assert_eq!(
                answer.value(),
                &[authored_named(q.model(), result)],
                "{owner} return"
            );
            let requirements = q
                .effective_usages_of_kind(authored_named(q.model(), owner), UsageKind::Requirement);
            complete(&requirements);
            assert!(
                requirements
                    .value()
                    .contains(&authored_named(q.model(), objective))
            );
        }
    }
    let cases = q.effective_usages_of_kind(
        authored_named(q.model(), "ReviewPlan"),
        agq_sysml_semantics::UsageKind::Case,
    );
    complete(&cases);
    assert!(
        cases
            .value()
            .contains(&authored_named(q.model(), "plannedReview"))
    );
    assert!(
        cases
            .value()
            .contains(&authored_named(q.model(), "plannedVerification"))
    );
    let verifications = q.effective_usages_of_kind(
        authored_named(q.model(), "ReviewPlan"),
        agq_sysml_semantics::UsageKind::VerificationCase,
    );
    complete(&verifications);
    assert_eq!(
        verifications.value(),
        &[authored_named(q.model(), "plannedVerification")]
    );
}

fn insert(
    project: &mut SourceProject,
    document: DocumentId,
    before: &str,
    text: &str,
) -> Arc<ProjectRevision> {
    let source = project.current().document(document).unwrap().source();
    let offset = source.find(before).unwrap() as u64;
    project
        .apply(
            project.current().revision(),
            [ProjectChange::Edit {
                document,
                edit: crate::syntax::TextEdit {
                    range: crate::syntax::ByteRange::new(offset, offset).unwrap(),
                    replacement: text.into(),
                },
            }],
        )
        .unwrap()
}

#[test]
#[ignore = "requires exact accepted KerML and Systems caches; never rebuilds standards"]
fn accepted_agentique_self_model_closes_queries_edits_and_matches_programmatic_semantics() {
    // Check both original inputs and compiled authority before the large KerML
    // restoration. Retain the opened handles so the checked inputs are consumed.
    let open_cache = |name| {
        let path = std::env::var_os(name)
            .unwrap_or_else(|| panic!("{name} is required for this requested acceptance gate"));
        let file = File::open(path)
            .unwrap_or_else(|error| panic!("{name} must name a readable original cache: {error}"));
        assert!(
            file.metadata().unwrap().is_file(),
            "{name} must name a regular cache file"
        );
        file
    };
    let kerml_cache = open_cache("AGENTIQUE_KERML_CACHE");
    let systems_cache = open_cache("AGENTIQUE_SYSTEMS_CACHE");
    agq_kerml_semantics::TrustedPublicationReceipt::checked_in("sysml-systems-operational-v2")
        .expect("the requested gate requires the independently accepted compiled Systems receipt");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let sources = VerifiedLibrarySet::load_from_directory(&root).unwrap();
    let kerml =
        Arc::new(CanonicalKermlStandardLibraries::restore_cache(kerml_cache, &sources).unwrap());
    let accepted = Arc::new(
        CanonicalSysmlSystemsLibrary::restore_cache(systems_cache, &sources, kerml).unwrap(),
    );
    // These independent fixtures do not need the retained authored history.
    // Finish them before allocating r0/r1 and keep only equivalence observations.
    accepted_case_roles(&accepted);
    accepted_view_metadata(&accepted);
    let programmatic = programmatic_semantics(&accepted);
    let mut project =
        SourceProject::with_accepted_sysml_standard_libraries(accepted.clone()).unwrap();
    let r1 = project
        .apply(
            project.current().revision(),
            super::agentique_self_model::DOCUMENTS
                .into_iter()
                .map(|(path, source)| ProjectChange::Add {
                    path: path.into(),
                    language: SourceLanguage::SysMl,
                    source: source.into(),
                }),
        )
        .unwrap();
    assert_revision(&r1, &accepted);
    architecture_invariants(&r1.sysml_queries().unwrap());
    accepted_trigger_and_message(&r1, &accepted);
    let original = rich_summary(&r1.sysml_queries().unwrap(), accepted.overlay().model());
    assert_eq!(original, programmatic.summary);
    assert_ne!(
        authored_named(r1.semantic_model(), "workspaceQuery"),
        programmatic.workspace_query
    );
    assert!(Arc::ptr_eq(
        &programmatic.dependency,
        r1.snapshot().immutable_dependency().unwrap()
    ));
    drop(programmatic);
    let document = r1.document_at("ModelingPlatform.sysml").unwrap().id();
    let retained_query = authored_named(r1.semantic_model(), "workspaceQuery");
    let r2 = insert(
        &mut project,
        document,
        "        attribute revisionNumber",
        "        port auditPort : ArchitectureContracts::SemanticQuery;\n",
    );
    assert_revision(&r2, &accepted);
    assert_eq!(
        authored_named(r2.semantic_model(), "workspaceQuery"),
        retained_query
    );
    let audit_port = authored_named(r2.semantic_model(), "auditPort");
    let r2_ports = {
        let q = r2.sysml_queries().unwrap();
        let ports = q.effective_ports(authored_named(q.model(), "IncrementalWorkspace"));
        complete(&ports);
        assert!(ports.value().contains(&audit_port));
        comparison_population(
            q.kerml(),
            accepted.overlay().model(),
            ports.value(),
            ComparisonOrder::Unordered,
        )
    };
    let r3 = insert(
        &mut project,
        document,
        "        attribute acceptedRevision",
        "        port auditedPort : ArchitectureContracts::SemanticQuery :>> ProjectWorkspace::auditPort;\n",
    );
    assert_revision(&r3, &accepted);
    assert_eq!(
        authored_named(r3.semantic_model(), "workspaceQuery"),
        retained_query
    );
    assert_eq!(authored_named(r3.semantic_model(), "auditPort"), audit_port);
    assert!(std::ptr::eq(
        r1.document_at("Contracts.sysml").unwrap(),
        r2.document_at("Contracts.sysml").unwrap()
    ));
    assert!(std::ptr::eq(
        r2.document_at("Contracts.sysml").unwrap(),
        r3.document_at("Contracts.sysml").unwrap()
    ));
    for (revision, expected_port) in [
        (&r2, audit_port),
        (&r3, authored_named(r3.semantic_model(), "auditedPort")),
    ] {
        let q = revision.sysml_queries().unwrap();
        let ports = q.effective_ports(authored_named(q.model(), "IncrementalWorkspace"));
        complete(&ports);
        assert!(ports.value().contains(&expected_port));
        if revision.revision() == r3.revision() {
            assert!(
                !ports.value().contains(&audit_port),
                "redefinition suppresses the inherited port"
            );
        }
    }
    let q2 = r2.sysml_queries().unwrap();
    let retained_ports = q2.effective_ports(authored_named(q2.model(), "IncrementalWorkspace"));
    complete(&retained_ports);
    assert_eq!(
        comparison_population(
            q2.kerml(),
            accepted.overlay().model(),
            retained_ports.value(),
            ComparisonOrder::Unordered,
        ),
        r2_ports
    );
    drop(q2);
    assert_eq!(
        rich_summary(&r1.sysml_queries().unwrap(), accepted.overlay().model()),
        original,
        "old revision retained"
    );
    assert!(
        !r2.document(document)
            .unwrap()
            .source()
            .contains("auditedPort")
    );
    assert!(Arc::ptr_eq(project.revision(r1.revision()).unwrap(), &r1));
    assert!(Arc::ptr_eq(
        project.accepted_sysml_standard_library().unwrap(),
        &accepted
    ));
    // r1/r2/r3 remain alive together below. Release the history's otherwise
    // unused empty r0 before constructing the independent sharing control.
    drop(project);
    {
        let second =
            SourceProject::with_accepted_sysml_standard_libraries(accepted.clone()).unwrap();
        assert!(Arc::ptr_eq(
            second.current().snapshot().immutable_dependency().unwrap(),
            r1.snapshot().immutable_dependency().unwrap()
        ));
    }
    std::thread::scope(|scope| {
        for revision in [&r1, &r2, &r3] {
            scope.spawn(move || {
                let q = revision.sysml_queries().unwrap();
                let ports = q.effective_ports(authored_named(q.model(), "IncrementalWorkspace"));
                complete(&ports);
                assert!(ports.value().contains(&retained_query));
            });
        }
    });
    println!(
        "Agentique self-model: {} documents, {} Complete mandatory references, producer closure Complete, semantic architecture invariants pass, programmatic equivalence pass, 3 immutable revisions, shared standards",
        r1.documents().count(),
        r1.references().len()
    );
}
