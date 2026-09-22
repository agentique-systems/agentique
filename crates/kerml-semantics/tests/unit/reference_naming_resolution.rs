use crate as agq_kerml_semantics;
include!("../common/namespace_fixture.rs");

struct Naming {
    reads_assertion: bool,
}
impl SemanticNamingExtension for Naming {
    fn naming_source(
        &self,
        q: &KerMlQueries<'_>,
        subject: ElementId,
    ) -> QueryResult<Option<Option<ElementId>>> {
        if subject != id(4) {
            return q
                .canonical_fact_evidence(FactKey::Element(subject))
                .map(|_| None);
        }
        if !self.reads_assertion {
            return q
                .canonical_fact_evidence(FactKey::Element(id(5)))
                .map(|_| Some(Some(id(2))));
        }
        let evidence = q.canonical_fact_evidence(FactKey::Property {
            element: id(5),
            property: p::SPECIALIZATION_GENERAL,
        });
        let target = q
            .model()
            .navigation_slot(id(5), p::REFERENCE_SUBSETTING_REFERENCED_FEATURE)
            .and_then(|slot| {
                slot.value().values().find_map(|value| match value {
                    Value::Reference(target) => Some(*target),
                    _ => None,
                })
            });
        evidence.map(|_| Some(target))
    }
}

fn fixture(target: u128, declared: Option<PropertyId>) -> ConstructionView {
    fixture_with_alias(target, declared, false)
}

fn fixture_with_alias(target: u128, declared: Option<PropertyId>, alias: bool) -> ConstructionView {
    let base = Snapshot::new(Arc::new(
        agq_kerml::registry_for_profile(agq_kerml::BaselineProfile::OPERATIONAL_V9).unwrap(),
    ));
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    f.create(1, c::PACKAGE);
    f.member(1, 12, 2, c::FEATURE, "work");
    f.member(1, 13, 3, c::FEATURE, "container");
    f.create(4, c::FEATURE);
    f.create(14, c::FEATURE_MEMBERSHIP);
    f.own(3, 14);
    f.changes.set(
        id(14),
        p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
        SlotValue::Ordered(vec![Value::Reference(id(4))]),
        origin(),
    );
    if let Some(property) = declared {
        f.value(4, property, Value::String("work".into()));
    }
    f.create(5, c::REFERENCE_SUBSETTING);
    f.own(4, 5);
    if target != 0 {
        f.value(
            5,
            p::REFERENCE_SUBSETTING_REFERENCED_FEATURE,
            Value::Reference(id(target)),
        );
    }
    if alias {
        f.create(15, c::MEMBERSHIP);
        f.own(3, 15);
        f.value(15, p::MEMBERSHIP_MEMBER_NAME, Value::String("work".into()));
        f.value(15, p::MEMBERSHIP_MEMBER_ELEMENT, Value::Reference(id(4)));
    }
    // Establish a historical KerML naming source independent of the composed
    // override. The new rule must not change it in a KerML-only context.
    f.create(6, c::REDEFINITION);
    f.own(4, 6);
    f.value(
        6,
        p::REDEFINITION_REDEFINING_FEATURE,
        Value::Reference(id(4)),
    );
    f.value(
        6,
        p::REDEFINITION_REDEFINED_FEATURE,
        Value::Reference(id(2)),
    );
    f.construction()
}

fn context(snapshot: &ConstructionView) -> SemanticContext<'_> {
    SemanticContext::for_construction(
        snapshot,
        SemanticOptions {
            baseline_profile: agq_kerml::BaselineProfile::OPERATIONAL_V9,
            ..Default::default()
        },
        BTreeSet::new(),
    )
    .unwrap()
}
fn name() -> QualifiedName {
    QualifiedName {
        absolute: false,
        segments: vec!["work".into()],
    }
}
fn resolve(q: &KerMlQueries<'_>) -> QueryResult<Vec<MemberMatch>> {
    q.lookup_relationship_target(id(5), p::REFERENCE_SUBSETTING_REFERENCED_FEATURE, &name())
}

#[test]
fn reference_resolution_cannot_use_its_own_endpoint_as_a_composed_naming_proof() {
    // Both the valid provisional endpoint and the previously self-bound cycle
    // resolve to the same independent membership. This models PerformActionUsage
    // and RequirementConstraintUsage without embedding SysML classes in KerML.
    for previous in [2, 4] {
        let snapshot = fixture(previous, None);
        let context = context(&snapshot)
            .with_naming_extension(
                "fixture-reference-naming/1",
                [7; 32],
                Arc::new(Naming {
                    reads_assertion: true,
                }),
            )
            .unwrap();
        let q = KerMlQueries::new(context);
        if previous == 4 {
            assert_eq!(
                q.effective_names(id(4)).completeness,
                Completeness::Incomplete
            );
        }
        let answer = resolve(&q);
        assert_eq!(
            answer.value,
            vec![MemberMatch {
                membership: id(12),
                element: id(2)
            }]
        );
        assert_eq!(
            answer.completeness,
            Completeness::Complete,
            "{:?}",
            answer.diagnostics
        );
        for (element, property) in [
            (id(4), p::ELEMENT_DECLARED_NAME),
            (id(4), p::ELEMENT_DECLARED_SHORT_NAME),
            (id(5), p::REFERENCE_SUBSETTING_REFERENCED_FEATURE),
        ] {
            assert!(
                answer
                    .search_dependencies
                    .contains(&SearchDependency::PropertySet { element, property })
            );
        }
        assert!(
            answer
                .search_dependencies
                .contains(&SearchDependency::ValidationRule(
                    "agq-composed-reference-naming/1"
                ))
        );
        let status = q
            .fork()
            .status_queries()
            .lookup_relationship_target_with_reads(
                id(5),
                p::REFERENCE_SUBSETTING_REFERENCED_FEATURE,
                &name(),
            );
        assert_eq!(status.outcome.value, answer.value);
        assert_eq!(status.outcome.completeness, answer.completeness);
        assert!(
            status
                .reads
                .search_dependencies
                .contains(&SearchDependency::PropertySet {
                    element: id(5),
                    property: p::REFERENCE_SUBSETTING_REFERENCED_FEATURE
                })
        );
    }
}

#[test]
fn explicitly_declared_names_and_independent_naming_sources_remain_candidates() {
    for declared in [
        Some(p::ELEMENT_DECLARED_NAME),
        Some(p::ELEMENT_DECLARED_SHORT_NAME),
    ] {
        let snapshot = fixture(2, declared);
        let q = KerMlQueries::new(
            context(&snapshot)
                .with_naming_extension(
                    "fixture-reference-naming/1",
                    [7; 32],
                    Arc::new(Naming {
                        reads_assertion: true,
                    }),
                )
                .unwrap(),
        );
        assert_eq!(
            resolve(&q).value,
            vec![MemberMatch {
                membership: id(14),
                element: id(4)
            }]
        );
    }
    let snapshot = fixture(2, None);
    let independent = KerMlQueries::new(
        context(&snapshot)
            .with_naming_extension(
                "fixture-reference-naming/1",
                [7; 32],
                Arc::new(Naming {
                    reads_assertion: false,
                }),
            )
            .unwrap(),
    );
    assert_eq!(
        resolve(&independent).value,
        vec![MemberMatch {
            membership: id(14),
            element: id(4)
        }]
    );
    let historical = resolve(&KerMlQueries::new(context(&snapshot)));
    assert_eq!(
        historical.value,
        vec![MemberMatch {
            membership: id(14),
            element: id(4)
        }]
    );
    assert!(historical.context.semantic_extensions.is_empty());
    assert!(
        !historical
            .search_dependencies
            .contains(&SearchDependency::ValidationRule(
                "agq-composed-reference-naming/1"
            ))
    );
}

#[test]
fn missing_endpoint_remains_incomplete_and_explicit_alias_keeps_its_own_name() {
    let missing = fixture(0, None);
    let q = KerMlQueries::new(
        context(&missing)
            .with_naming_extension(
                "fixture-reference-naming/1",
                [7; 32],
                Arc::new(Naming {
                    reads_assertion: true,
                }),
            )
            .unwrap(),
    );
    let answer = resolve(&q);
    assert_eq!(
        answer.value,
        vec![MemberMatch {
            membership: id(12),
            element: id(2)
        }]
    );
    assert_eq!(answer.completeness, Completeness::Incomplete);
    assert!(
        answer
            .search_dependencies
            .contains(&SearchDependency::ValidationRule(
                "agq-composed-reference-naming/1"
            ))
    );
    assert!(
        answer
            .search_dependencies
            .contains(&SearchDependency::PropertySet {
                element: id(5),
                property: p::REFERENCE_SUBSETTING_REFERENCED_FEATURE,
            })
    );
    let aliased = fixture_with_alias(2, None, true);
    let q = KerMlQueries::new(
        context(&aliased)
            .with_naming_extension(
                "fixture-reference-naming/1",
                [7; 32],
                Arc::new(Naming {
                    reads_assertion: true,
                }),
            )
            .unwrap(),
    );
    // Cache ordinary and historical broad-exclusion populations first. The new
    // owning-only cache key must retain the independent, explicit alias name.
    assert_eq!(
        q.lookup_member(id(3), "work", MemberAccess::All)
            .value
            .len(),
        2
    );
    assert!(
        q.namespace_members_excluding(id(3), MemberAccess::All, Some(id(4)))
            .value
            .is_empty()
    );
    let answer = resolve(&q);
    assert_eq!(
        answer.value,
        vec![MemberMatch {
            membership: id(15),
            element: id(4)
        }]
    );
    assert_eq!(answer.completeness, Completeness::Complete);
    assert_eq!(resolve(&q.fork()), answer);
}
