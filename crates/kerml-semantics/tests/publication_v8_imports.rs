include!("common/namespace_fixture.rs");
use agq_kerml::BaselineProfile as P;

fn fixture(profile: P) -> Fixture {
    let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(profile).unwrap()));
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    for n in [1, 2, 3, 4] {
        f.create(n, c::NAMESPACE);
    }
    f
}
fn queries(snapshot: &Snapshot, profile: P) -> KerMlQueries<'_> {
    KerMlQueries::new(
        SemanticContext::for_snapshot(
            snapshot,
            SemanticOptions {
                baseline_profile: profile,
                ..Default::default()
            },
            Default::default(),
        )
        .unwrap(),
    )
}

#[test]
fn owned_collisions_use_comparable_metaclasses_and_preserve_historical_formal_population() {
    for profile in [P::PublishedKerMl10, P::OPERATIONAL_V7, P::OPERATIONAL_V8] {
        let mut f = fixture(profile);
        f.member(1, 11, 12, c::FEATURE, "same");
        f.member(2, 21, 22, c::FEATURE, "same");
        f.member(2, 23, 24, c::CLASS, "same");
        f.import(1, 31, 2, false);
        let snapshot = f.finish();
        let q = queries(&snapshot, profile);
        assert!(!q.memberships_distinguishable(id(21), id(21)).value);
        let result = q.imported_memberships(id(1), MemberAccess::All);
        assert_eq!(
            result.completeness,
            Completeness::Complete,
            "{:?}",
            result.diagnostics
        );
        let members: BTreeSet<_> = result.value.iter().map(|m| m.membership).collect();
        assert_eq!(
            members.contains(&id(21)),
            !profile.corrects_import_collisions()
        );
        assert!(
            members.contains(&id(23)),
            "same name, incomparable metaclasses"
        );
        assert_eq!(q.owning_relationship(id(22)).value, Some(id(21)));
    }
}

#[test]
fn collisions_are_symmetric_deduplicated_and_do_not_choose_an_import_winner() {
    for reversed in [false, true] {
        let mut f = fixture(P::OPERATIONAL_V8);
        f.member(2, 900, 22, c::FEATURE, "collision");
        f.member(3, 13, 32, c::FEATURE, "collision");
        f.member(2, 910, 24, c::FEATURE, "first");
        f.member(3, 14, 34, c::FEATURE, "second");
        for (r, target) in if reversed {
            [(41, 3), (42, 2)]
        } else {
            [(41, 2), (42, 3)]
        } {
            f.import(1, r, target, false);
        }
        f.import(1, 43, 910, true); // Same canonical Membership through another Import.
        let snapshot = f.finish();
        let q = queries(&snapshot, P::OPERATIONAL_V8);
        let result = q.imported_memberships(id(1), MemberAccess::All);
        assert_eq!(
            result.completeness,
            Completeness::Complete,
            "{:?}",
            result.diagnostics
        );
        assert_eq!(
            result
                .value
                .iter()
                .map(|m| m.membership)
                .collect::<Vec<_>>(),
            if reversed {
                vec![id(14), id(910)]
            } else {
                vec![id(910), id(14)]
            }
        );
        assert_eq!(
            q.namespace_members(id(1), MemberAccess::All).value,
            result.value
        );
    }
}

#[test]
fn cyclic_recursive_imports_aliases_and_visibility_keep_canonical_memberships() {
    let mut f = fixture(P::OPERATIONAL_V8);
    f.member(2, 21, 22, c::FEATURE, "original");
    f.member(2, 23, 24, c::FEATURE, "hidden");
    f.visibility(23, "private");
    f.create(25, c::MEMBERSHIP);
    f.own(3, 25);
    f.value(25, p::MEMBERSHIP_MEMBER_ELEMENT, Value::Reference(id(22)));
    f.value(25, p::MEMBERSHIP_MEMBER_NAME, Value::String("alias".into()));
    f.import(1, 31, 2, false);
    f.import(2, 32, 3, false);
    f.import(3, 33, 1, false);
    f.value(31, p::IMPORT_IS_RECURSIVE, Value::Boolean(true));
    let snapshot = f.finish();
    let q = queries(&snapshot, P::OPERATIONAL_V8);
    let result = q.imported_memberships(id(1), MemberAccess::All);
    assert_eq!(
        result.completeness,
        Completeness::Complete,
        "{:?}",
        result.diagnostics
    );
    assert_eq!(
        result
            .value
            .iter()
            .map(|m| m.membership)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([id(21), id(25)])
    );
    assert_eq!(
        q.lookup_member(id(1), "alias", MemberAccess::All).value[0].membership,
        id(25)
    );
}

#[test]
fn import_cycles_do_not_reimport_the_importing_namespaces_owned_members() {
    let mut f = fixture(P::OPERATIONAL_V8);
    f.member(1, 11, 12, c::FEATURE, "local");
    f.member(2, 21, 22, c::FEATURE, "remote");
    f.import(1, 31, 2, false);
    f.import(2, 32, 1, false);
    let snapshot = f.finish();
    let q = queries(&snapshot, P::OPERATIONAL_V8);
    let imports = q.imported_memberships(id(1), MemberAccess::All);
    assert_eq!(imports.completeness, Completeness::Complete);
    assert_eq!(
        imports.value,
        vec![MemberMatch {
            membership: id(21),
            element: id(22)
        }]
    );
}
