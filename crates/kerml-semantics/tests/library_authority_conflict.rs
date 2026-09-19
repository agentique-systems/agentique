include!("common/namespace_fixture.rs");

fn edge(f: &mut Fixture, relationship: u128, class: MetaclassId, source: u128, target: u128) {
    f.create(relationship, class);
    let (specific, general) = match class {
        c::SUBCLASSIFICATION => (
            p::SUBCLASSIFICATION_SUBCLASSIFIER,
            p::SUBCLASSIFICATION_SUPERCLASSIFIER,
        ),
        c::REDEFINITION => (
            p::REDEFINITION_REDEFINING_FEATURE,
            p::REDEFINITION_REDEFINED_FEATURE,
        ),
        c::FEATURE_TYPING => (p::FEATURE_TYPING_TYPED_FEATURE, p::FEATURE_TYPING_TYPE),
        c::SUBSETTING => (
            p::SUBSETTING_SUBSETTING_FEATURE,
            p::SUBSETTING_SUBSETTED_FEATURE,
        ),
        _ => panic!("unsupported witness edge"),
    };
    f.value(relationship, specific, Value::Reference(id(source)));
    f.value(relationship, general, Value::Reference(id(target)));
    f.own(source, relationship);
}

#[test]
fn two_explicit_type_paths_require_a_common_redefinition_before_publication() {
    for profile in [
        agq_kerml::BaselineProfile::PublishedKerMl10,
        agq_kerml::BaselineProfile::OPERATIONAL_V1,
        agq_kerml::BaselineProfile::OPERATIONAL_V2,
    ] {
        for repaired in [false, true] {
            let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(profile).unwrap()));
            let changes = base.change_set();
            let mut f = Fixture {
                base,
                changes,
                owned: BTreeMap::new(),
            };
            f.create(1, c::NAMESPACE);
            for (membership, target, name) in
                [(102, 2, "Cobalt"), (103, 3, "Quartz"), (104, 4, "Jasper")]
            {
                f.member(1, membership, target, c::CLASS, name);
            }
            f.member(2, 110, 10, c::FEATURE, "tone");
            f.member(3, 111, 11, c::FEATURE, "tone");
            f.member(4, 112, 12, c::FEATURE, "tone");
            edge(&mut f, 200, c::SUBCLASSIFICATION, 3, 2);
            edge(&mut f, 201, c::SUBCLASSIFICATION, 4, 2);
            edge(&mut f, 202, c::REDEFINITION, 11, 10);
            edge(&mut f, 203, c::REDEFINITION, 12, 10);
            f.member(1, 120, 20, c::FEATURE, "cells");
            edge(&mut f, 204, c::FEATURE_TYPING, 20, 4);
            f.member(1, 121, 21, c::FEATURE, "edges");
            edge(&mut f, 205, c::FEATURE_TYPING, 21, 3);
            edge(&mut f, 206, c::SUBSETTING, 21, 20);
            if repaired {
                f.member(1, 130, 30, c::CLASS, "Combined");
                f.member(30, 131, 31, c::FEATURE, "tone");
                edge(&mut f, 207, c::SUBCLASSIFICATION, 30, 3);
                edge(&mut f, 208, c::SUBCLASSIFICATION, 30, 4);
                edge(&mut f, 209, c::REDEFINITION, 31, 11);
                edge(&mut f, 210, c::REDEFINITION, 31, 12);
                f.value(205, p::FEATURE_TYPING_TYPE, Value::Reference(id(30)));
            }
            let snapshot = f.finish();
            let q = KerMlQueries::new(
                SemanticContext::for_snapshot(
                    &snapshot,
                    SemanticOptions {
                        baseline_profile: profile,
                        ..Default::default()
                    },
                    Default::default(),
                )
                .unwrap(),
            );
            let lookup = q.lookup_member(id(21), "tone", MemberAccess::All);
            let targets: Vec<_> = lookup
                .value
                .iter()
                .map(|m| m.element)
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            assert_eq!(
                targets,
                if repaired {
                    vec![id(31)]
                } else {
                    vec![id(11), id(12)]
                }
            );
            assert_eq!(lookup.completeness, Completeness::Complete);
            let validation = q.validate_namespace_distinguishability(id(21));
            assert_eq!(
                validation.completeness,
                if repaired {
                    Completeness::Complete
                } else {
                    Completeness::Invalid
                }
            );
            assert!(
                !lookup
                    .explanations
                    .values()
                    .flatten()
                    .any(|p| p.rule == Rule::OperationalRedefinitionTargetV1)
            );
            println!(
                "AUTHORITY_WITNESS {}",
                serde_json::json!({"profile":profile.id(),"explicit_common_redefinition":repaired,
                "targets":targets.iter().map(|id|id.to_string()).collect::<Vec<_>>(),
                "lookup_completeness":format!("{:?}",lookup.completeness),
                "validation":format!("{:?}",validation.completeness),
                "positive_dependencies":lookup.positive_dependencies.iter().map(|d|format!("{d:?}")).collect::<Vec<_>>(),
                "search_dependencies":lookup.search_dependencies.iter().map(|d|format!("{d:?}")).collect::<Vec<_>>() })
            );
        }
    }
}
