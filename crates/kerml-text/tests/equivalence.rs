use agq_kerml::{classes as c, properties as p};
use agq_kerml_text::{Document, syntax::ReferenceKind};
use agq_kernel::{metamodel::*, provenance::*, value::*, *};
use std::{collections::BTreeMap, sync::Arc};

fn authored() -> DeclaredOrigin {
    DeclaredOrigin::Authored { source: None }
}

#[test]
fn programmatic_and_textual_models_have_equivalent_canonical_records() {
    let document = Document::new(include_str!("fixtures/equivalence.kerml")).unwrap();
    let textual = document.current();
    assert!(
        textual.validate_slice().is_ok(),
        "{:?}",
        textual.diagnostics()
    );
    let q = textual.queries();
    let mut text_ids = BTreeMap::from([("root", textual.root())]);
    for (owner, name) in [
        ("root", "N"),
        ("N", "Base"),
        ("Base", "f"),
        ("N", "Derived"),
        ("Derived", "g"),
    ] {
        let id = q.lookup_declared_member(text_ids[owner], name).value[0];
        text_ids.insert(name, id);
    }
    for (name, membership) in [
        ("N", "mN"),
        ("Base", "mBase"),
        ("f", "mf"),
        ("Derived", "mDerived"),
        ("g", "mg"),
    ] {
        text_ids.insert(
            membership,
            q.owning_relationship(text_ids[name]).value.unwrap(),
        );
    }
    for assertion in textual.references() {
        let name = match assertion.kind {
            ReferenceKind::Specialization => "specialization",
            ReferenceKind::Typing => "typing",
            ReferenceKind::Redefinition => "redefinition",
            _ => unreachable!(),
        };
        text_ids.insert(name, assertion.relationship);
    }
    // Independent programmatic construction, with a different ID allocation and
    // no source or parser object involved in the canonical transaction.
    let base = Snapshot::new(Arc::new(agq_kerml::registry().unwrap()));
    let registry = base.model().registry();
    let mut change = base.change_set();
    let classes = [
        ("root", c::NAMESPACE),
        ("N", c::NAMESPACE),
        ("Base", c::FEATURE),
        ("f", c::FEATURE),
        ("Derived", c::TYPE),
        ("g", c::FEATURE),
        ("mN", c::OWNING_MEMBERSHIP),
        ("mBase", c::OWNING_MEMBERSHIP),
        ("mf", c::FEATURE_MEMBERSHIP),
        ("mDerived", c::OWNING_MEMBERSHIP),
        ("mg", c::FEATURE_MEMBERSHIP),
        ("specialization", c::SPECIALIZATION),
        ("typing", c::FEATURE_TYPING),
        ("redefinition", c::REDEFINITION),
    ];
    let ids: BTreeMap<_, _> = classes
        .iter()
        .map(|(name, _)| (*name, ElementId::new()))
        .collect();
    for (name, class) in classes {
        let id = ids[name];
        change.create(id, class, authored());
        for property in registry
            .effective_properties(class)
            .unwrap()
            .filter(|p| !p.derived && p.multiplicity.lower > 0)
        {
            let value = match registry.storage_kind(property.value_kind).unwrap() {
                ValueKind::Boolean => Value::Boolean(property.id == p::FEATURE_IS_UNIQUE),
                ValueKind::String => {
                    assert_eq!(property.id, p::ELEMENT_ELEMENT_ID);
                    Value::String(id.to_string())
                }
                ValueKind::Enumeration(domain) => Value::Enumeration(
                    *registry
                        .enumeration(domain)
                        .unwrap()
                        .literals
                        .iter()
                        .find(|(_, name)| name.as_str() == "public")
                        .unwrap()
                        .0,
                ),
                ValueKind::Reference(_) => continue,
                _ => panic!("unexpected normative required slot"),
            };
            change.set(id, property.id, SlotValue::Scalar(value), authored());
        }
    }
    for name in ["N", "Base", "f", "Derived", "g"] {
        change.set(
            ids[name],
            p::ELEMENT_DECLARED_NAME,
            SlotValue::Scalar(Value::String(name.into())),
            authored(),
        );
    }
    for (owner, relationships) in [
        ("root", vec!["mN"]),
        ("N", vec!["mBase", "mDerived"]),
        ("Base", vec!["mf"]),
        ("Derived", vec!["specialization", "mg"]),
        ("g", vec!["typing", "redefinition"]),
    ] {
        change.set(
            ids[owner],
            p::ELEMENT_OWNED_RELATIONSHIP,
            SlotValue::Ordered(
                relationships
                    .into_iter()
                    .map(|n| Value::Reference(ids[n]))
                    .collect(),
            ),
            authored(),
        );
    }
    for (membership, member) in [
        ("mN", "N"),
        ("mBase", "Base"),
        ("mf", "f"),
        ("mDerived", "Derived"),
        ("mg", "g"),
    ] {
        change.set(
            ids[membership],
            p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
            SlotValue::Ordered(vec![Value::Reference(ids[member])]),
            authored(),
        );
    }
    for (relationship, class, specific, general) in [
        ("specialization", c::SPECIALIZATION, "Derived", "Base"),
        ("typing", c::FEATURE_TYPING, "g", "Base"),
        ("redefinition", c::REDEFINITION, "g", "f"),
    ] {
        for (property, target) in [
            (p::SPECIALIZATION_SPECIFIC, specific),
            (p::SPECIALIZATION_GENERAL, general),
        ] {
            let effective = registry
                .resolve_property(class, property)
                .unwrap()
                .unwrap()
                .id;
            change.set(
                ids[relationship],
                effective,
                SlotValue::Scalar(Value::Reference(ids[target])),
                authored(),
            );
        }
    }
    let programmatic = base.apply(&change).unwrap();
    assert_eq!(
        canonical(&programmatic, &ids),
        canonical(textual.snapshot(), &text_ids)
    );
    // Drop the entire parser/frontend: a snapshot owns only kernel records,
    // descriptor values and source provenance IDs/ranges.
    let detached = textual.snapshot().clone();
    drop(document);
    assert_eq!(detached.model().len(), programmatic.model().len());
    fn send_sync<T: Send + Sync>() {}
    send_sync::<Snapshot>();
    send_sync::<ElementRecord>();
}

fn canonical(
    snapshot: &Snapshot,
    ids: &BTreeMap<&str, ElementId>,
) -> BTreeMap<String, (MetaclassId, BTreeMap<PropertyId, String>)> {
    let labels: BTreeMap<_, _> = ids.iter().map(|(name, id)| (*id, *name)).collect();
    assert_eq!(snapshot.model().len(), ids.len());
    snapshot
        .model()
        .elements()
        .map(|record| {
            let slots = record
                .slots()
                .filter(|(p, _)| *p != p::ELEMENT_ELEMENT_ID)
                .map(|(property, slot)| {
                    let values: Vec<_> = slot
                        .value()
                        .values()
                        .map(|v| match v {
                            Value::Reference(id) => format!("ref:{}", labels[id]),
                            v => format!("{v:?}"),
                        })
                        .collect();
                    (
                        property,
                        format!("{:?}:{values:?}", std::mem::discriminant(slot.value())),
                    )
                })
                .collect();
            (labels[&record.id()].into(), (record.metaclass(), slots))
        })
        .collect()
}
