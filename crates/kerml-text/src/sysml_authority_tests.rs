//! Exact declared authority witnesses, deliberately not an accepted publication.
use super::*;
use agq_kernel::{MetaclassId, derived::PropertyState};
use agq_standard_libraries::LibraryDocument;
use serde_json::json;

fn owned_members(query: &KerMlQueries<'_>, scope: ElementId) -> Vec<(ElementId, ElementId)> {
    let answer = query.owned_relationships(scope);
    assert_eq!(answer.completeness, Completeness::Complete, "{answer:?}");
    assert!(answer.positive_dependencies.contains(&FactKey::Property {
        element: scope,
        property: p::ELEMENT_OWNED_RELATIONSHIP,
    }));
    answer
        .value
        .into_iter()
        .filter(|&relationship| {
            query
                .model()
                .registry()
                .is_subtype(
                    query.model().element(relationship).unwrap().metaclass(),
                    c::OWNING_MEMBERSHIP,
                )
                .unwrap()
        })
        .map(|membership| {
            let answer = query.member(membership);
            assert_eq!(answer.completeness, Completeness::Complete, "{answer:?}");
            assert!(answer.positive_dependencies.contains(&FactKey::Property {
                element: membership,
                property: p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
            }));
            (
                membership,
                answer.value.expect("one canonical owned member"),
            )
        })
        .collect()
}

fn declared_name(model: &ModelView, element: ElementId) -> Option<String> {
    match model
        .property_state(element, p::ELEMENT_DECLARED_NAME)
        .unwrap()
    {
        PropertyState::Computed(slot) => match slot.value() {
            SlotValue::Scalar(Value::String(name)) => Some(name.clone()),
            other => panic!("invalid declared name: {other:?}"),
        },
        PropertyState::Absent => None,
        other => panic!("incomplete declared name: {other:?}"),
    }
}

fn owned_path(
    query: &KerMlQueries<'_>,
    roots: &[ElementId],
    path: &[&str],
) -> Vec<(ElementId, ElementId)> {
    let mut scopes = roots.to_vec();
    let mut result = Vec::new();
    for segment in path {
        result = scopes
            .iter()
            .flat_map(|&scope| owned_members(query, scope))
            .filter(|&(_, element)| {
                declared_name(query.model(), element).as_deref() == Some(*segment)
            })
            .collect();
        assert!(result.len() <= 1, "ambiguous owned declaration {path:?}");
        scopes = result.iter().map(|&(_, element)| element).collect();
        if scopes.is_empty() {
            break;
        }
    }
    result
}

fn assert_source(
    draft: &LibraryDraft,
    documents: &[(&LibraryDocument, production::Document)],
    element: ElementId,
) -> serde_json::Value {
    let origin = &draft.source_map()[&FactKey::Element(element)];
    let (source, syntax) = documents
        .iter()
        .find(|(source, _)| source.document() == origin.document)
        .unwrap();
    assert_eq!(origin.revision, source.revision());
    assert_eq!(
        draft.candidate().model().element(element).unwrap().origin(),
        &Origin::Declared(source.origin())
    );
    let syntax_node = origin.syntax_node.expect("canonical source node identity");
    let node = syntax
        .nodes()
        .find(|node| node.id() == syntax_node)
        .unwrap();
    assert_eq!(node.range(), origin.range);
    json!({
        "path": source.path(), "sha256": source.sha256(),
        "library_id": source.library().to_string(),
        "document_id": origin.document.to_string(),
        "revision_id": origin.revision.to_string(),
        "syntax_node_id": syntax_node.to_string(),
        "byte_range": [origin.range.start(), origin.range.end()],
        "origin": "Declared::StandardLibrary",
    })
}

fn exact_subject(
    draft: &LibraryDraft,
    query: &KerMlQueries<'_>,
    documents: &[(&LibraryDocument, production::Document)],
    path: &[&str],
    expected: MetaclassId,
) -> (ElementId, serde_json::Value) {
    let members = owned_path(query, draft.roots(), path);
    let [(membership, element)] = members.as_slice() else {
        panic!("expected unique owned declaration {path:?}: {members:?}");
    };
    let model = query.model();
    assert_eq!(model.element(*element).unwrap().metaclass(), expected);
    let PropertyState::Computed(visibility) = model
        .property_state(*membership, p::MEMBERSHIP_VISIBILITY)
        .unwrap()
    else {
        panic!("incomplete membership visibility");
    };
    let SlotValue::Scalar(Value::Enumeration(literal)) = visibility.value() else {
        panic!("invalid membership visibility");
    };
    let agq_kernel::metamodel::ValueKind::Enumeration(domain) = model
        .registry()
        .property(p::MEMBERSHIP_VISIBILITY)
        .unwrap()
        .value_kind
    else {
        panic!("invalid visibility descriptor");
    };
    assert_eq!(
        model.registry().enumeration(domain).unwrap().literals[literal],
        "public"
    );
    let source = assert_source(draft, documents, *element);
    let membership_source = assert_source(draft, documents, *membership);
    assert_eq!(source["library_id"], membership_source["library_id"]);
    let declared_origin = model.element(*element).unwrap().origin();
    assert_eq!(
        model
            .navigation_slot(*element, p::ELEMENT_DECLARED_NAME)
            .unwrap()
            .origin(),
        declared_origin
    );
    assert_eq!(visibility.origin(), declared_origin);
    (
        *element,
        json!({
            "path": path.join("::"), "element_id": element.to_string(),
            "metaclass": model.registry().class(expected).unwrap().name,
            "membership_id": membership.to_string(), "visibility": "public",
            "owned_path_status": "Complete", "unique": true,
            "source": source, "membership_source": membership_source,
        }),
    )
}

#[test]
fn exact_systems_authority_targets_have_complete_declared_owned_path_witnesses() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let sources = VerifiedLibrarySet::load_from_directory(&root).unwrap();
    let documents: Vec<_> = sources
        .documents()
        .filter(|source| source.language() == LibraryLanguage::SysMl)
        .map(|source| {
            let syntax = production::parse_sysml_with_profile(
                production::SysmlSyntaxProfile::OperationalV1,
                source.document(),
                source.revision(),
                source.source(),
                Default::default(),
            )
            .unwrap();
            assert!(syntax.is_complete(), "{}", source.path());
            assert_eq!(syntax.source().as_bytes(), source.source().as_bytes());
            (source, syntax)
        })
        .collect();
    assert_eq!(documents.len(), 21);
    let inputs: Vec<_> = documents
        .iter()
        .map(|(source, syntax)| SourceInput {
            syntax,
            library: Some(source),
            sysml: true,
        })
        .collect();
    // Empty canonical graph in the full combined descriptor registry. This is
    // only a declared construction fixture, with no accepted KerML dependency,
    // endpoint refinement, producer execution or publication claim.
    let base = Snapshot::new(Arc::new(
        agq_sysml::registry_for_profile(BaselineProfile::OPERATIONAL_V9).unwrap(),
    ));
    let draft = construction::construct_on(
        &inputs,
        &Default::default(),
        BaselineProfile::OPERATIONAL_V9,
        base,
        None,
    )
    .unwrap();
    assert_eq!(draft.roots().len(), 21);
    assert!(!draft.candidate().obligations().is_empty());
    let query = KerMlQueries::new(
        SemanticContext::for_construction(draft.candidate(), options(), BTreeSet::new()).unwrap(),
    );
    let mut findings = Vec::new();
    for (id, missing, existing, class, expected_id) in [
        (
            "SYSML20-PUB-001",
            ["Views", "Viewpoint"],
            ["Views", "ViewpointCheck"],
            s::VIEWPOINT_DEFINITION,
            "8c8a3d17-aa0d-50dc-bb3b-b5b41b330b5d",
        ),
        (
            "SYSML20-PUB-002",
            ["Views", "viewpoints"],
            ["Views", "viewpointChecks"],
            s::VIEWPOINT_USAGE,
            "29b6d0ae-8a25-55d0-ac30-9d153e62ddba",
        ),
        (
            "SYSML20-PUB-003",
            ["Connections", "BinaryConnections"],
            ["Connections", "BinaryConnection"],
            s::CONNECTION_DEFINITION,
            "6f256a75-d031-5390-90d0-24ba7666497b",
        ),
    ] {
        // Establish the parent package independently, so an empty path cannot
        // hide an absent namespace or a skipped incomplete ownership read.
        exact_subject(
            &draft,
            &query,
            &documents,
            &missing[..1],
            c::LIBRARY_PACKAGE,
        );
        assert!(owned_path(&query, draft.roots(), &missing).is_empty());
        let (element, subject) = exact_subject(&draft, &query, &documents, &existing, class);
        assert_eq!(element.to_string(), expected_id);
        let mut witness = json!({
            "finding": id, "formal_target": missing.join("::"),
            "formal_target_owned_path": "CompleteAbsent", "subject": subject,
        });
        if class == s::CONNECTION_DEFINITION {
            let mut ends = Vec::new();
            for (membership, member) in owned_members(&query, element) {
                if !query
                    .model()
                    .registry()
                    .is_subtype(
                        query.model().element(membership).unwrap().metaclass(),
                        c::FEATURE_MEMBERSHIP,
                    )
                    .unwrap()
                {
                    continue;
                }
                let PropertyState::Computed(is_end) = query
                    .model()
                    .property_state(member, p::FEATURE_IS_END)
                    .unwrap()
                else {
                    panic!("incomplete declared end flag");
                };
                if is_end.value() == &SlotValue::Scalar(Value::Boolean(true)) {
                    let name = declared_name(query.model(), member).unwrap();
                    let (found, end) = exact_subject(
                        &draft,
                        &query,
                        &documents,
                        &["Connections", "BinaryConnection", &name],
                        s::REFERENCE_USAGE,
                    );
                    assert_eq!(found, member);
                    assert_eq!(
                        is_end.origin(),
                        query.model().element(member).unwrap().origin()
                    );
                    ends.push(end);
                }
            }
            assert_eq!(
                ends.iter()
                    .map(|end| end["path"].as_str().unwrap())
                    .collect::<BTreeSet<_>>(),
                BTreeSet::from([
                    "Connections::BinaryConnection::source",
                    "Connections::BinaryConnection::target"
                ])
            );
            assert_eq!(ends.len(), 2);
            witness["antecedent"] =
                json!({"declared_owned_end_count": 2, "complete": true, "ends": ends});
        } else {
            witness["antecedent"] =
                json!({"exact_declared_metaclass": true, "additional_condition": null});
        }
        findings.push(witness);
    }
    let (_, nested_usage) = exact_subject(
        &draft,
        &query,
        &documents,
        &["Views", "View", "viewpointSatisfactions"],
        s::VIEWPOINT_USAGE,
    );
    println!(
        "AUTHORITY_DECLARED_WITNESS {}",
        json!({
            "fixture": "unpublished all-21-document declared candidate on empty combined-registry graph",
            "accepted_kerml_dependency": false, "producer_closure": "not-run",
            "global_namespace_completeness_claim": false, "mandatory_reference_closure_claim": false,
            "documents": documents.len(), "findings": findings,
            "additional_viewpoint_usage": nested_usage,
        })
    );
}
