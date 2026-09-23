//! Items::Touches has named cross ReferenceUsages, each with [0..*], and
//! two inherited end redefinitions. Preserve those distinctions from Flows.
use super::*;

#[test]
fn item_ends_with_named_cross_multiplicity_and_multiple_redefinitions_close() {
    connection_end_fixture(
        |f, occurrence, _| {
            let (_, roles) = corpus_anchor_fixture_complete(true);
            let item = roles[&StandardSysmlRole::Item];
            f.relation(
                item,
                occurrence.as_u128(),
                277_000,
                kc::SUBCLASSIFICATION,
                kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
            );
            f.relation(
                roles[&StandardSysmlRole::Items],
                item,
                277_001,
                kc::FEATURE_TYPING,
                kp::FEATURE_TYPING_TYPE,
            );
            // The existing ConnectionBase supplies the two spatial ends; this
            // second supertype supplies thisOccurrence and thatOccurrence.
            f.create(77_000, kc::ASSOCIATION, "TemporalBase");
            f.member(1, 77_000, 177_000, kc::OWNING_MEMBERSHIP);
            f.relation(
                77_000,
                occurrence.as_u128(),
                277_002,
                kc::SUBCLASSIFICATION,
                kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
            );
            for (end, name) in [(77_001, "thisOccurrence"), (77_002, "thatOccurrence")] {
                f.create(end, kc::FEATURE, name);
                f.value(end, kp::FEATURE_IS_END, Value::Boolean(true));
                f.member(77_000, end, end + 100_000, kc::FEATURE_MEMBERSHIP);
                f.relation(
                    end,
                    occurrence.as_u128(),
                    end + 210_000,
                    kc::FEATURE_TYPING,
                    kp::FEATURE_TYPING_TYPE,
                );
            }
            f.create(77_010, sc::CONNECTION_DEFINITION, "Touches");
            f.member(1, 77_010, 177_010, kc::OWNING_MEMBERSHIP);
            for (supertype, relation) in [(75_000, 297_010), (77_000, 297_011)] {
                f.relation(
                    77_010,
                    supertype,
                    relation,
                    kc::SUBCLASSIFICATION,
                    kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
                );
            }
            for (end, name, cross, cross_name, spatial, temporal) in [
                (
                    77_011,
                    "touchedItemToo",
                    77_021,
                    "touchesToo",
                    75_001,
                    77_001,
                ),
                (77_012, "touchedItem", 77_022, "touches", 75_002, 77_002),
            ] {
                f.create(end, sc::ITEM_USAGE, name);
                f.member(77_010, end, end + 100_000, kc::FEATURE_MEMBERSHIP);
                f.value(end, kp::FEATURE_IS_END, Value::Boolean(true));
                f.value(end, kp::FEATURE_IS_COMPOSITE, Value::Boolean(true));
                f.create(cross, sc::REFERENCE_USAGE, cross_name);
                f.member(end, cross, cross + 100_000, kc::OWNING_MEMBERSHIP);
                f.relation(
                    end,
                    spatial,
                    end + 200_000,
                    kc::REDEFINITION,
                    kp::REDEFINITION_REDEFINED_FEATURE,
                );
                f.relation(
                    end,
                    temporal,
                    end + 210_000,
                    kc::REDEFINITION,
                    kp::REDEFINITION_REDEFINED_FEATURE,
                );
                let multiplicity = cross + 100;
                f.create(multiplicity, kc::MULTIPLICITY_RANGE, "");
                f.member(
                    cross,
                    multiplicity,
                    multiplicity + 100_000,
                    kc::OWNING_MEMBERSHIP,
                );
                for (literal, class) in [
                    (cross + 200, kc::LITERAL_INTEGER),
                    (cross + 300, kc::LITERAL_INFINITY),
                ] {
                    f.changes.create(id(literal), class, f.origin.clone());
                    for property in f
                        .base
                        .model()
                        .registry()
                        .effective_properties(class)
                        .unwrap()
                    {
                        if property.derived || property.multiplicity.lower == 0 {
                            continue;
                        }
                        let value = match f
                            .base
                            .model()
                            .registry()
                            .storage_kind(property.value_kind)
                            .unwrap()
                        {
                            ValueKind::Boolean => Value::Boolean(false),
                            ValueKind::String => Value::String(String::new()),
                            ValueKind::Integer => Value::Integer(0.into()),
                            ValueKind::Reference(_) => continue,
                            other => panic!("literal fixture domain {other:?}"),
                        };
                        f.changes.set(
                            id(literal),
                            property.id,
                            SlotValue::Scalar(value),
                            f.origin.clone(),
                        );
                    }
                    f.member(
                        multiplicity,
                        literal,
                        literal + 100_000,
                        kc::OWNING_MEMBERSHIP,
                    );
                    let result = literal + 1000;
                    f.create(result, kc::FEATURE, "");
                    set_enum(f, result, kp::FEATURE_DIRECTION, "out");
                    f.member(
                        literal,
                        result,
                        result + 100_000,
                        kc::RETURN_PARAMETER_MEMBERSHIP,
                    );
                    f.value(
                        result + 100_000,
                        kp::RELATIONSHIP_IS_IMPLIED,
                        Value::Boolean(true),
                    );
                    f.value(
                        literal,
                        kp::ELEMENT_IS_IMPLIED_INCLUDED,
                        Value::Boolean(true),
                    );
                    f.changes.clear(id(result), kp::ELEMENT_DECLARED_NAME);
                }
                f.changes.clear(id(multiplicity), kp::ELEMENT_DECLARED_NAME);
            }
        },
        &[(77_011, Some(77_021)), (77_012, Some(77_022))],
    );
}
