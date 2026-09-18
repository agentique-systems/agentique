use agq_kerml::{classes as c, properties as p};
use agq_kerml_semantics::*;
use agq_kernel::{metamodel::ValueKind, provenance::*, value::*, *};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

fn id(n: u128) -> ElementId {
    ElementId::from_u128(n)
}
fn origin() -> DeclaredOrigin {
    DeclaredOrigin::Authored { source: None }
}
struct Fixture {
    base: Snapshot,
    changes: ChangeSet,
    owned: BTreeMap<ElementId, Vec<Value>>,
}
impl Fixture {
    fn new() -> Self {
        let base = Snapshot::new(Arc::new(agq_kerml::registry().unwrap()));
        let changes = base.change_set();
        Self {
            base,
            changes,
            owned: BTreeMap::new(),
        }
    }
    fn create(&mut self, n: u128, class: MetaclassId) {
        self.changes.create(id(n), class, origin());
        for property in self
            .base
            .model()
            .registry()
            .effective_properties(class)
            .unwrap()
        {
            if property.derived || property.multiplicity.lower == 0 {
                continue;
            }
            let value = match self
                .base
                .model()
                .registry()
                .storage_kind(property.value_kind)
                .unwrap()
            {
                ValueKind::Boolean => Value::Boolean(false),
                ValueKind::String => Value::String(n.to_string()),
                ValueKind::Enumeration(domain) => Value::Enumeration(
                    *self
                        .base
                        .model()
                        .registry()
                        .enumeration(domain)
                        .unwrap()
                        .literals
                        .iter()
                        .find(|(_, name)| name.as_str() == "public")
                        .unwrap()
                        .0,
                ),
                ValueKind::Reference(_) => continue,
                _ => panic!("fixture domain"),
            };
            self.changes
                .set(id(n), property.id, SlotValue::Scalar(value), origin());
        }
    }
    fn value(&mut self, n: u128, property: PropertyId, value: Value) {
        self.changes
            .set(id(n), property, SlotValue::Scalar(value), origin());
    }
    fn own(&mut self, owner: u128, relationship: u128) {
        self.owned
            .entry(id(owner))
            .or_default()
            .push(Value::Reference(id(relationship)));
    }
    fn member(
        &mut self,
        owner: u128,
        membership: u128,
        target: u128,
        class: MetaclassId,
        name: &str,
    ) {
        self.create(target, class);
        self.value(target, p::ELEMENT_DECLARED_NAME, Value::String(name.into()));
        self.create(
            membership,
            if class == c::FEATURE {
                c::FEATURE_MEMBERSHIP
            } else {
                c::OWNING_MEMBERSHIP
            },
        );
        self.own(owner, membership);
        self.changes.set(
            id(membership),
            p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
            SlotValue::Ordered(vec![Value::Reference(id(target))]),
            origin(),
        );
    }
    fn import(&mut self, owner: u128, import: u128, target: u128, membership: bool) {
        self.create(
            import,
            if membership {
                c::MEMBERSHIP_IMPORT
            } else {
                c::NAMESPACE_IMPORT
            },
        );
        self.own(owner, import);
        self.value(
            import,
            if membership {
                p::MEMBERSHIP_IMPORT_IMPORTED_MEMBERSHIP
            } else {
                p::NAMESPACE_IMPORT_IMPORTED_NAMESPACE
            },
            Value::Reference(id(target)),
        );
    }
    fn visibility(&mut self, element: u128, name: &str) {
        let ValueKind::Enumeration(domain) = self
            .base
            .model()
            .registry()
            .property(p::MEMBERSHIP_VISIBILITY)
            .unwrap()
            .value_kind
        else {
            unreachable!()
        };
        let literal = *self
            .base
            .model()
            .registry()
            .enumeration(domain)
            .unwrap()
            .literals
            .iter()
            .find(|(_, n)| n.as_str() == name)
            .unwrap()
            .0;
        self.value(
            element,
            p::MEMBERSHIP_VISIBILITY,
            Value::Enumeration(literal),
        );
    }
    fn finish(mut self) -> Snapshot {
        for (owner, values) in self.owned {
            self.changes.set(
                owner,
                p::ELEMENT_OWNED_RELATIONSHIP,
                SlotValue::Ordered(values),
                origin(),
            );
        }
        self.base.apply(&self.changes).unwrap()
    }
}
fn queries(s: &Snapshot) -> KerMlQueries<'_> {
    KerMlQueries::new(
        SemanticContext::for_snapshot(s, Default::default(), Default::default()).unwrap(),
    )
}
fn lookup(s: &Snapshot, scope: u128, name: &str) -> QueryResult<Resolution> {
    queries(s).resolve_name(
        id(scope),
        &QualifiedName {
            absolute: false,
            segments: name.split("::").map(str::to_string).collect(),
        },
        c::ELEMENT,
        false,
    )
}
fn basic() -> Fixture {
    let mut f = Fixture::new();
    f.create(1, c::NAMESPACE);
    f.member(1, 10, 2, c::NAMESPACE, "Source");
    f.member(1, 11, 3, c::NAMESPACE, "User");
    f.member(2, 20, 4, c::TYPE, "Thing");
    f
}

#[test]
fn membership_namespace_imports_aliases_shadowing_and_ambiguity() {
    for membership in [false, true] {
        let mut f = basic();
        f.import(3, 30, if membership { 20 } else { 2 }, membership);
        let s = f.finish();
        let answer = lookup(&s, 3, "Thing");
        assert_eq!(answer.value, Resolution::Resolved(id(4)));
        assert_eq!(answer.completeness, Completeness::Complete);
        assert!(
            answer
                .positive_dependencies
                .contains(&FactKey::Element(id(30)))
        );
        assert!(
            answer
                .search_dependencies
                .contains(&SearchDependency::ImportSet { namespace: id(3) })
        );
        assert_eq!(lookup(&s, 3, "absent").value, Resolution::Unresolved);
    }
    let mut f = basic();
    f.import(3, 30, 2, false);
    f.member(3, 21, 5, c::TYPE, "Thing");
    assert_eq!(
        lookup(&f.finish(), 3, "Thing").value,
        Resolution::Resolved(id(5))
    );
    let mut f = basic();
    f.create(31, c::MEMBERSHIP);
    f.own(3, 31);
    f.value(31, p::MEMBERSHIP_MEMBER_ELEMENT, Value::Reference(id(4)));
    f.value(31, p::MEMBERSHIP_MEMBER_NAME, Value::String("Alias".into()));
    let s = f.finish();
    assert_eq!(lookup(&s, 3, "Alias").value, Resolution::Resolved(id(4)));
    let membership = queries(&s).resolve_name(
        id(3),
        &QualifiedName {
            absolute: false,
            segments: vec!["Alias".into()],
        },
        c::MEMBERSHIP,
        true,
    );
    assert_eq!(membership.value, Resolution::Resolved(id(31)));
    let mut f = basic();
    f.member(1, 12, 5, c::NAMESPACE, "Other");
    f.member(5, 22, 6, c::TYPE, "Thing");
    f.import(3, 30, 2, false);
    f.import(3, 31, 5, false);
    assert_eq!(
        lookup(&f.finish(), 3, "Thing").value,
        Resolution::Ambiguous(vec![id(4), id(6)])
    );
}

#[test]
fn cycles_recursive_imports_and_visibility_preserve_negative_dependencies() {
    let mut f = basic();
    f.member(2, 21, 5, c::NAMESPACE, "Nested");
    f.member(5, 22, 6, c::TYPE, "Deep");
    f.import(3, 30, 2, false);
    f.value(30, p::IMPORT_IS_RECURSIVE, Value::Boolean(true));
    f.import(2, 31, 3, false);
    f.visibility(20, "private");
    let s = f.finish();
    assert_eq!(lookup(&s, 3, "Deep").value, Resolution::Resolved(id(6)));
    let missing = lookup(&s, 3, "Thing");
    assert_eq!(missing.value, Resolution::Unresolved);
    assert!(
        missing
            .search_dependencies
            .contains(&SearchDependency::NamespaceMembers { namespace: id(2) })
    );
    assert!(
        missing
            .search_dependencies
            .contains(&SearchDependency::PropertySet {
                element: id(20),
                property: p::MEMBERSHIP_VISIBILITY
            })
    );
    let mut change = s.change_set();
    change.set(
        id(20),
        p::MEMBERSHIP_VISIBILITY,
        s.model()
            .element(id(21))
            .unwrap()
            .slot(p::MEMBERSHIP_VISIBILITY)
            .unwrap()
            .value()
            .clone(),
        origin(),
    );
    let public = s.apply(&change).unwrap();
    let found = lookup(&public, 3, "Thing");
    assert_eq!(found.value, Resolution::Resolved(id(4)));
    let mut change = public.change_set();
    change.set(
        id(3),
        p::ELEMENT_OWNED_RELATIONSHIP,
        SlotValue::Ordered(vec![]),
        origin(),
    );
    change.remove(id(30));
    let removed = public.apply(&change).unwrap();
    assert_eq!(lookup(&removed, 3, "Thing").value, Resolution::Unresolved);
    assert!(
        found
            .search_dependencies
            .contains(&SearchDependency::ImportSet { namespace: id(3) })
    );
}

#[test]
fn inherited_protected_members_and_explicit_project_boundaries() {
    let mut f = basic();
    f.member(4, 21, 5, c::FEATURE, "protectedFeature");
    f.visibility(21, "protected");
    f.member(3, 22, 6, c::TYPE, "Derived");
    f.create(30, c::SPECIALIZATION);
    f.value(30, p::SPECIALIZATION_SPECIFIC, Value::Reference(id(6)));
    f.value(30, p::SPECIALIZATION_GENERAL, Value::Reference(id(4)));
    f.own(6, 30);
    f.create(100, c::NAMESPACE);
    f.member(100, 110, 101, c::TYPE, "External");
    let s = f.finish();
    assert_eq!(
        lookup(&s, 6, "protectedFeature").value,
        Resolution::Resolved(id(5))
    );
    assert_eq!(
        lookup(&s, 3, "Derived::protectedFeature").value,
        Resolution::Unresolved
    );
    assert_eq!(lookup(&s, 3, "External").value, Resolution::Unresolved);
    let context = SemanticContext::for_snapshot(&s, Default::default(), Default::default())
        .unwrap()
        .with_available_roots(BTreeMap::from([(id(1), BTreeSet::from([id(1), id(100)]))]))
        .unwrap();
    let q = KerMlQueries::new(context);
    let external = q.resolve_name(
        id(3),
        &QualifiedName {
            absolute: true,
            segments: vec!["External".into()],
        },
        c::TYPE,
        false,
    );
    assert_eq!(external.value, Resolution::Resolved(id(101)));
    assert!(
        external
            .search_dependencies
            .contains(&SearchDependency::ProjectRoots { root: id(1) })
    );
    assert_eq!(
        q.resolve_name(
            id(100),
            &QualifiedName {
                absolute: true,
                segments: vec!["User".into()]
            },
            c::NAMESPACE,
            false
        )
        .value,
        Resolution::Unresolved
    );
}

#[test]
fn relationship_context_uses_redefinition_supertype_and_previous_chain_link() {
    let mut f = basic();
    f.member(4, 21, 5, c::FEATURE, "end");
    f.member(3, 22, 6, c::TYPE, "Derived");
    f.member(6, 23, 7, c::FEATURE, "end");
    f.create(30, c::SPECIALIZATION);
    f.value(30, p::SPECIALIZATION_SPECIFIC, Value::Reference(id(6)));
    f.value(30, p::SPECIALIZATION_GENERAL, Value::Reference(id(4)));
    f.own(6, 30);
    f.create(31, c::REDEFINITION);
    f.value(
        31,
        p::REDEFINITION_REDEFINING_FEATURE,
        Value::Reference(id(7)),
    );
    f.value(
        31,
        p::REDEFINITION_REDEFINED_FEATURE,
        Value::Reference(id(5)),
    );
    f.own(7, 31);
    f.member(7, 24, 8, c::FEATURE, "nested");
    f.member(6, 25, 9, c::FEATURE, "chain");
    for (link, target) in [(32, 7), (33, 8)] {
        f.create(link, c::FEATURE_CHAINING);
        f.value(
            link,
            p::FEATURE_CHAINING_CHAINING_FEATURE,
            Value::Reference(id(target)),
        );
        f.own(9, link);
    }
    let s = f.finish();
    let q = queries(&s);
    let name = |s: &str| QualifiedName {
        absolute: false,
        segments: vec![s.into()],
    };
    assert_eq!(
        q.lookup_relationship_target(id(31), p::REDEFINITION_REDEFINED_FEATURE, &name("end"))
            .value,
        vec![MemberMatch {
            membership: id(21),
            element: id(5)
        }]
    );
    assert_eq!(
        q.lookup_relationship_target(id(32), p::FEATURE_CHAINING_CHAINING_FEATURE, &name("end"))
            .value,
        vec![MemberMatch {
            membership: id(23),
            element: id(7)
        }]
    );
    assert_eq!(
        q.lookup_relationship_target(
            id(33),
            p::FEATURE_CHAINING_CHAINING_FEATURE,
            &name("nested")
        )
        .value,
        vec![MemberMatch {
            membership: id(24),
            element: id(8)
        }]
    );
}

#[test]
fn insertion_and_alias_change_have_answer_affecting_search_evidence() {
    let mut f = basic();
    f.import(3, 30, 2, false);
    let before = f.finish();
    let miss = lookup(&before, 3, "NewAlias");
    assert_eq!(miss.value, Resolution::Unresolved);
    assert!(
        miss.search_dependencies
            .contains(&SearchDependency::NamespaceMembers { namespace: id(2) })
    );
    let mut change = before.change_set();
    change.create(id(31), c::MEMBERSHIP, origin());
    change.set(
        id(31),
        p::ELEMENT_ELEMENT_ID,
        SlotValue::Scalar(Value::String("31".into())),
        origin(),
    );
    change.set(
        id(31),
        p::ELEMENT_IS_IMPLIED_INCLUDED,
        SlotValue::Scalar(Value::Boolean(false)),
        origin(),
    );
    change.set(
        id(31),
        p::RELATIONSHIP_IS_IMPLIED,
        SlotValue::Scalar(Value::Boolean(false)),
        origin(),
    );
    change.set(
        id(31),
        p::MEMBERSHIP_VISIBILITY,
        before
            .model()
            .element(id(20))
            .unwrap()
            .slot(p::MEMBERSHIP_VISIBILITY)
            .unwrap()
            .value()
            .clone(),
        origin(),
    );
    change.set(
        id(31),
        p::MEMBERSHIP_MEMBER_ELEMENT,
        SlotValue::Scalar(Value::Reference(id(4))),
        origin(),
    );
    change.set(
        id(31),
        p::MEMBERSHIP_MEMBER_NAME,
        SlotValue::Scalar(Value::String("NewAlias".into())),
        origin(),
    );
    change.set(
        id(2),
        p::ELEMENT_OWNED_RELATIONSHIP,
        SlotValue::Ordered(vec![Value::Reference(id(20)), Value::Reference(id(31))]),
        origin(),
    );
    let after = before.apply(&change).unwrap();
    let hit = lookup(&after, 3, "NewAlias");
    assert_eq!(hit.value, Resolution::Resolved(id(4)));
    assert!(
        hit.search_dependencies
            .contains(&SearchDependency::PropertySet {
                element: id(31),
                property: p::MEMBERSHIP_MEMBER_NAME
            })
    );
    let mut change = after.change_set();
    change.set(
        id(31),
        p::MEMBERSHIP_MEMBER_NAME,
        SlotValue::Scalar(Value::String("Renamed".into())),
        origin(),
    );
    assert_eq!(
        lookup(&after.apply(&change).unwrap(), 3, "NewAlias").value,
        Resolution::Unresolved
    );
}

#[test]
fn renamed_inherited_redefinition_removes_a_diamond_peer_before_name_selection() {
    let mut f = basic();
    f.member(4, 21, 5, c::FEATURE, "oldName");
    for (membership, ty, name) in [(22, 6, "Left"), (23, 7, "Right"), (24, 8, "Diamond")] {
        f.member(3, membership, ty, c::TYPE, name);
    }
    f.member(6, 25, 9, c::FEATURE, "newName");
    for (relationship, specific, general) in [(30, 6, 4), (31, 7, 4), (32, 8, 6), (33, 8, 7)] {
        f.create(relationship, c::SPECIALIZATION);
        f.value(
            relationship,
            p::SPECIALIZATION_SPECIFIC,
            Value::Reference(id(specific)),
        );
        f.value(
            relationship,
            p::SPECIALIZATION_GENERAL,
            Value::Reference(id(general)),
        );
        f.own(specific, relationship);
    }
    f.create(34, c::REDEFINITION);
    f.value(
        34,
        p::REDEFINITION_REDEFINING_FEATURE,
        Value::Reference(id(9)),
    );
    f.value(
        34,
        p::REDEFINITION_REDEFINED_FEATURE,
        Value::Reference(id(5)),
    );
    f.own(9, 34);
    let s = f.finish();
    assert_eq!(lookup(&s, 8, "oldName").value, Resolution::Unresolved);
    assert_eq!(lookup(&s, 8, "newName").value, Resolution::Resolved(id(9)));
}

#[test]
fn deep_inheritance_cyclic_imports_and_simultaneous_queries_are_finite() {
    let mut f = Fixture::new();
    f.create(1, c::NAMESPACE);
    for n in 0..64 {
        f.member(1, 100 + n, 1000 + n, c::NAMESPACE, &format!("N{n}"));
        f.import(1000 + n, 2000 + n, 1000 + (n + 1) % 64, false);
    }
    f.member(1000, 4000, 4001, c::TYPE, "imported");
    for n in 0..256 {
        f.member(1, 10000 + n, 20000 + n, c::TYPE, &format!("T{n}"));
        if n > 0 {
            f.create(30000 + n, c::SPECIALIZATION);
            f.value(
                30000 + n,
                p::SPECIALIZATION_SPECIFIC,
                Value::Reference(id(20000 + n)),
            );
            f.value(
                30000 + n,
                p::SPECIALIZATION_GENERAL,
                Value::Reference(id(20000 + n - 1)),
            );
            f.own(20000 + n, 30000 + n);
        }
    }
    f.member(20000, 40000, 40001, c::FEATURE, "inherited");
    let s = f.finish();
    let q = queries(&s);
    let name = |s: &str| QualifiedName {
        absolute: false,
        segments: vec![s.into()],
    };
    assert_eq!(
        q.resolve_name(id(1063), &name("imported"), c::TYPE, false)
            .value,
        Resolution::Resolved(id(4001))
    );
    assert_eq!(
        q.resolve_name(id(20255), &name("inherited"), c::FEATURE, false)
            .value,
        Resolution::Resolved(id(40001))
    );
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|_| {
                scope.spawn(|| {
                    let missing = q.resolve_name(id(1063), &name("absent"), c::TYPE, false);
                    let inherited =
                        q.resolve_name(id(20255), &name("inherited"), c::FEATURE, false);
                    assert_eq!(missing.value, Resolution::Unresolved);
                    assert_eq!(inherited.completeness, Completeness::Complete);
                    assert_eq!(inherited.value, Resolution::Resolved(id(40001)));
                    assert_eq!(inherited.context, q.context().clone());
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
    });
}
