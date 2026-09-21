use agq_kerml::{classes as c, properties as p};
use agq_kerml_semantics::*;
use agq_kernel::{metamodel::ValueKind, provenance::*, value::*, *};
// Included by independently scoped test binaries with different builder needs.
#[allow(unused_imports)]
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
// Shared by independently scoped test binaries, each using different builders.
#[allow(dead_code)]
impl Fixture {
    fn construction(mut self) -> agq_kernel::ConstructionView {
        for (owner, values) in self.owned {
            self.changes.set(
                owner,
                p::ELEMENT_OWNED_RELATIONSHIP,
                SlotValue::Ordered(values),
                origin(),
            );
        }
        self.base.preview(&self.changes).unwrap()
    }
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
        let registry = self.base.model().registry();
        if !registry.supports_slot_storage(property).unwrap() {
            let descriptor = registry.property(property).unwrap();
            let Value::Reference(target) = value else {
                panic!("fixture reference occurrence");
            };
            self.changes.link(
                AssociationOccurrenceId::new(),
                descriptor.association.unwrap(),
                BTreeMap::from([
                    (property, target),
                    (*descriptor.opposite_ends.first().unwrap(), id(n)),
                ]),
                BTreeMap::new(),
                origin(),
            );
            return;
        }
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
        self.enumeration(element, p::MEMBERSHIP_VISIBILITY, name);
    }
    fn enumeration(&mut self, element: u128, property: PropertyId, name: &str) {
        let ValueKind::Enumeration(domain) = self
            .base
            .model()
            .registry()
            .property(property)
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
        self.value(element, property, Value::Enumeration(literal));
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
