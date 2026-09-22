//! Independent programmatic equivalent of the authored Part vertical.
use agq_kerml::{classes as c, properties as p};
use agq_kernel::{
    ElementId, Snapshot,
    provenance::DeclaredOrigin,
    value::{SlotValue, Value},
};
use agq_sysml::{classes as s, properties as sp};

pub fn construct(base: Snapshot) -> Result<(Snapshot, [ElementId; 5]), agq_kernel::ModelError> {
    let mut changes = base.change_set();
    let origin = DeclaredOrigin::Authored { source: None };
    let id = ElementId::from_u128;
    let classes = [
        (1, c::NAMESPACE),
        (2, s::PART_DEFINITION),
        (3, s::PART_DEFINITION),
        (4, s::PART_DEFINITION),
        (5, s::PART_USAGE),
        (11, c::OWNING_MEMBERSHIP),
        (12, c::OWNING_MEMBERSHIP),
        (13, c::OWNING_MEMBERSHIP),
        (14, c::FEATURE_MEMBERSHIP),
        (21, c::FEATURE_TYPING),
        (22, c::SUBCLASSIFICATION),
    ];
    for (number, class) in classes {
        changes.create(id(number), class, origin.clone());
        changes.set(
            id(number),
            p::ELEMENT_ELEMENT_ID,
            SlotValue::Scalar(Value::String(id(number).to_string())),
            origin.clone(),
        );
        // Explicit fixture defaults, checked against effective descriptor storage.
        for property in [
            p::ELEMENT_IS_IMPLIED_INCLUDED,
            p::RELATIONSHIP_IS_IMPLIED,
            p::TYPE_IS_ABSTRACT,
            p::TYPE_IS_SUFFICIENT,
            p::FEATURE_IS_COMPOSITE,
            p::FEATURE_IS_CONSTANT,
            p::FEATURE_IS_DERIVED,
            p::FEATURE_IS_END,
            p::FEATURE_IS_ORDERED,
            p::FEATURE_IS_PORTION,
            sp::DEFINITION_IS_VARIATION,
            sp::USAGE_IS_VARIATION,
            sp::OCCURRENCE_DEFINITION_IS_INDIVIDUAL,
            sp::OCCURRENCE_USAGE_IS_INDIVIDUAL,
        ] {
            if base
                .model()
                .registry()
                .resolve_property(class, property)
                .unwrap()
                .is_some_and(|p| !p.derived)
            {
                changes.set(
                    id(number),
                    property,
                    SlotValue::Scalar(Value::Boolean(false)),
                    origin.clone(),
                );
            }
        }
        if base
            .model()
            .registry()
            .is_subtype(class, c::MEMBERSHIP)
            .unwrap()
        {
            let agq_kernel::metamodel::ValueKind::Enumeration(domain) = base
                .model()
                .registry()
                .property(p::MEMBERSHIP_VISIBILITY)
                .unwrap()
                .value_kind
            else {
                unreachable!()
            };
            let literal = *base
                .model()
                .registry()
                .enumeration(domain)
                .unwrap()
                .literals
                .iter()
                .find(|(_, name)| name.as_str() == "public")
                .unwrap()
                .0;
            changes.set(
                id(number),
                p::MEMBERSHIP_VISIBILITY,
                SlotValue::Scalar(Value::Enumeration(literal)),
                origin.clone(),
            );
        }
    }
    for (number, name) in [
        (2, "Engine"),
        (3, "Vehicle"),
        (4, "SportsCar"),
        (5, "engine"),
    ] {
        changes.set(
            id(number),
            p::ELEMENT_DECLARED_NAME,
            SlotValue::Scalar(Value::String(name.into())),
            origin.clone(),
        );
    }
    changes.set(
        id(5),
        p::FEATURE_IS_UNIQUE,
        SlotValue::Scalar(Value::Boolean(true)),
        origin.clone(),
    );
    changes.set(
        id(5),
        p::FEATURE_IS_COMPOSITE,
        SlotValue::Scalar(Value::Boolean(true)),
        origin.clone(),
    );
    for (owner, property, targets) in [
        (1, p::ELEMENT_OWNED_RELATIONSHIP, vec![11, 12, 13]),
        (11, p::RELATIONSHIP_OWNED_RELATED_ELEMENT, vec![2]),
        (12, p::RELATIONSHIP_OWNED_RELATED_ELEMENT, vec![3]),
        (13, p::RELATIONSHIP_OWNED_RELATED_ELEMENT, vec![4]),
        (3, p::ELEMENT_OWNED_RELATIONSHIP, vec![14]),
        (14, p::RELATIONSHIP_OWNED_RELATED_ELEMENT, vec![5]),
        (5, p::ELEMENT_OWNED_RELATIONSHIP, vec![21]),
        (4, p::ELEMENT_OWNED_RELATIONSHIP, vec![22]),
    ] {
        changes.set(
            id(owner),
            property,
            SlotValue::Ordered(
                targets
                    .into_iter()
                    .map(|target| Value::Reference(id(target)))
                    .collect(),
            ),
            origin.clone(),
        );
    }
    changes.set(
        id(21),
        p::FEATURE_TYPING_TYPE,
        SlotValue::Scalar(Value::Reference(id(2))),
        origin.clone(),
    );
    changes.set(
        id(21),
        p::FEATURE_TYPING_TYPED_FEATURE,
        SlotValue::Scalar(Value::Reference(id(5))),
        origin.clone(),
    );
    changes.set(
        id(22),
        p::SUBCLASSIFICATION_SUBCLASSIFIER,
        SlotValue::Scalar(Value::Reference(id(4))),
        origin.clone(),
    );
    changes.set(
        id(22),
        p::SUBCLASSIFICATION_SUPERCLASSIFIER,
        SlotValue::Scalar(Value::Reference(id(3))),
        origin,
    );
    let snapshot = base.apply(&changes)?;
    Ok((snapshot, [id(1), id(2), id(3), id(4), id(5)]))
}
