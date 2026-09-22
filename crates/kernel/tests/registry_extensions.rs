mod common;
use agq_kernel::{metamodel::*, *};
use common::*;
use std::collections::BTreeSet;

const EXTRA: MetaclassId = MetaclassId::from_u128(9100);
const EXTRA_NAME: PropertyId = PropertyId::from_u128(9101);

fn input() -> DescriptorSet {
    let (classes, properties) = descriptors();
    DescriptorSet {
        models: vec![model_descriptor()],
        classes,
        properties,
        ..Default::default()
    }
}

#[test]
fn new_class_redefinition_preserves_existing_property_resolution() {
    let base = MetamodelRegistry::from_descriptors(input()).unwrap();
    let mut extension = input();
    extension
        .classes
        .push(class(EXTRA, "Extension", &[PART_DEF]));
    let mut renamed = property(
        EXTRA_NAME,
        "extensionName",
        EXTRA,
        ValueKind::String,
        Multiplicity::OPTIONAL,
    );
    renamed.redefines.insert(NAME);
    extension.properties.push(renamed);
    let extension = MetamodelRegistry::from_descriptors(extension).unwrap();
    extension.require_extension_of(&base).unwrap();
    assert_eq!(
        extension.resolve_property(EXTRA, NAME).unwrap().unwrap().id,
        EXTRA_NAME
    );
    for old in base.classes() {
        for property in base.properties() {
            assert_eq!(
                extension.resolve_property(old.id, property.id),
                base.resolve_property(old.id, property.id)
            );
        }
    }
}

#[test]
fn structurally_valid_replacements_and_new_properties_on_old_classes_are_rejected() {
    let base = MetamodelRegistry::from_descriptors(input()).unwrap();
    for case in 0..6 {
        let mut extension = input();
        match case {
            0 => extension.properties.retain(|p| p.id != NAME),
            1 => {
                extension
                    .classes
                    .iter_mut()
                    .find(|c| c.id == PART_DEF)
                    .unwrap()
                    .name = "Changed".into()
            }
            2 => {
                extension
                    .properties
                    .iter_mut()
                    .find(|p| p.id == NAME)
                    .unwrap()
                    .ordered = true
            }
            3 => extension.properties.push(property(
                EXTRA_NAME,
                "newOldClassSlot",
                TYPE,
                ValueKind::String,
                Multiplicity::OPTIONAL,
            )),
            4 => extension.models[0].uri = "urn:replaced-authority".into(),
            _ => {
                extension.sources.insert(
                    DescriptorId::Class(TYPE),
                    DescriptorSource {
                        specification: "Added provenance on an old descriptor".into(),
                        version: "1".into(),
                        artifact_uri: "urn:test".into(),
                        sha256: "different".into(),
                        external_id: "type".into(),
                        byte_range: [0, 1],
                    },
                );
            }
        }
        let extension = MetamodelRegistry::from_descriptors(extension).unwrap();
        assert!(
            matches!(
                extension.require_extension_of(&base),
                Err(MetamodelError::IncompatibleExtension { .. })
            ),
            "case {case}"
        );
    }
    assert_eq!(base.class(PART_DEF).unwrap().name, "PartDefinition");
}

#[test]
fn primitive_and_enumeration_domains_cannot_be_rebound() {
    let primitive = PrimitiveDomainId::from_u128(9200);
    let enumeration = EnumerationId::from_u128(9201);
    let literal = EnumerationLiteralId::from_u128(9202);
    let mut descriptors = input();
    descriptors.primitives.push(PrimitiveDescriptor {
        id: primitive,
        name: "Domain".into(),
        representation: PrimitiveRepresentation::Integer,
    });
    descriptors.enumerations.push(EnumerationDescriptor {
        id: enumeration,
        name: "Choice".into(),
        package: vec![],
        metamodel: MM,
        literals: [(literal, "first".into())].into(),
    });
    let base = MetamodelRegistry::from_descriptors(descriptors.clone()).unwrap();
    descriptors.primitives[0].representation = PrimitiveRepresentation::Real;
    assert!(
        MetamodelRegistry::from_descriptors(descriptors.clone())
            .unwrap()
            .require_extension_of(&base)
            .is_err()
    );
    descriptors.primitives[0].representation = PrimitiveRepresentation::Integer;
    descriptors.enumerations[0]
        .literals
        .insert(EnumerationLiteralId::from_u128(9203), "second".into());
    assert!(
        MetamodelRegistry::from_descriptors(descriptors)
            .unwrap()
            .require_extension_of(&base)
            .is_err()
    );
    assert_eq!(
        base.enumeration(enumeration)
            .unwrap()
            .literals
            .keys()
            .copied()
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([literal])
    );
}
