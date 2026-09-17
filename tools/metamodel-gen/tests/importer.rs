use agq_metamodel_gen::{canonical_json, cross_check, ir::*, sha256, xmi};

const FOUNDATION: &str = include_str!("fixtures/foundation.xmi");
const CROSS_CHECK: &str = include_str!("fixtures/cross-check.xmi");

fn source(xml: &str) -> Source {
    Source {
        specification: "Fixture".into(),
        version: "1.0".into(),
        metamodel_uri: "urn:agentique:fixture:1".into(),
        artifact_uri: "urn:agentique:fixture:1/model.xmi".into(),
        sha256: sha256(xml.as_bytes()),
    }
}

fn import(xml: &str) -> agq_metamodel_gen::Result<Metamodel> {
    xmi::import(xml, source(xml), &Default::default())
}

#[test]
fn same_artifact_produces_identical_ir_and_keys() {
    let a = import(FOUNDATION).unwrap();
    let b = import(FOUNDATION).unwrap();
    assert_eq!(a, b);
    assert_eq!(canonical_json(&a).unwrap(), canonical_json(&b).unwrap());
    let key = &a.classifiers["left-A"].entity.key;
    assert_eq!(key.uuid(), b.classifiers["left-A"].entity.key.uuid());
    assert_eq!(key.metaclass_id().unwrap().as_u128(), key.uuid().as_u128());
    assert_ne!(key.uuid(), a.classifiers["right-A"].entity.key.uuid());
    let mut changed = key.clone();
    changed.source.version = "2.0".into();
    assert_ne!(key.uuid(), changed.uuid());
    changed = key.clone();
    changed.source.sha256 = "0".repeat(64);
    assert_ne!(key.uuid(), changed.uuid());
    changed = key.clone();
    changed.kind = Kind::Property;
    assert_ne!(key.uuid(), changed.uuid());
    assert!(key.property_id().is_err());
    let p = &a.properties["C-peer"].entity.key;
    assert_eq!(p.property_id().unwrap().as_u128(), p.uuid().as_u128());
    assert!(p.metaclass_id().is_err());
}

#[test]
fn descriptor_key_v1_golden_encoding() {
    let key = DescriptorKey {
        source: Source {
            specification: "S".into(),
            version: "1".into(),
            metamodel_uri: "urn:m".into(),
            artifact_uri: "urn:a".into(),
            sha256: "abc".into(),
        },
        package_path: vec!["P".into(), "Q".into()],
        external_id: "x-id".into(),
        kind: Kind::Class,
    };
    assert_eq!(
        String::from_utf8(key.encoded()).unwrap(),
        r#"["agentique-descriptor-key/1","S","1","urn:m","urn:a","abc",["P","Q"],"x-id","class"]"#
    );
    // Fixed expected UUID additionally protects namespace/hash algorithm changes.
    assert_eq!(
        key.uuid().to_string(),
        "dace05fb-c70c-5ff9-b4d6-065bc430a0bf"
    );
}

#[test]
fn inheritance_multiplicity_containment_and_uninterpreted_semantics_survive() {
    let model = import(FOUNDATION).unwrap();
    assert_eq!(model.classifiers["C"].generalizations, ["left-A", "B"]);
    assert!(model.classifiers["left-A"].is_abstract);
    assert_eq!(
        model.classifiers["C"].entity.qualified_path,
        ["Fixture", "Left", "C"]
    );
    let items = &model.properties["left-A-items"];
    assert_eq!(items.lower, 0);
    assert_eq!(items.upper, Upper::Unlimited);
    assert!(items.is_ordered && items.is_read_only && !items.is_unique);
    assert_eq!(items.aggregation, Aggregation::Composite);
    let refined = &model.properties["C-refined"];
    assert_eq!((refined.lower, &refined.upper), (2, &Upper::Finite(4)));
    assert!(refined.is_derived && refined.is_derived_union);
    assert_eq!(refined.redefines, ["left-A-items"]);
    assert_eq!(refined.subsets, ["left-A-items"]);
    let peer = &model.properties["C-peer"];
    assert_eq!((peer.lower, &peer.upper), (1, &Upper::Finite(1)));
    assert_eq!(peer.aggregation, Aggregation::None);
    assert_eq!(peer.opposite_ends, ["peers-back"]);
    assert_eq!(model.properties["peers-back"].owner, "peers");
    assert!(
        model.classifiers["C"]
            .retained
            .iter()
            .any(|n| n.tag == "ownedRule"
                && n.children[0].attributes["body"] == "self.refined->notEmpty()")
    );
}

#[test]
fn unresolved_metaclass_and_wrong_kind_references_fail() {
    let bad = FOUNDATION.replacen(
        "<type xmi:idref=\"left-A\"/>",
        "<type xmi:idref=\"missing\"/>",
        1,
    );
    assert!(
        import(&bad)
            .unwrap_err()
            .contains("unresolved reference missing")
    );
    let bad = FOUNDATION.replacen(
        "<type xmi:idref=\"left-A\"/>",
        "<type xmi:idref=\"left\"/>",
        1,
    );
    assert!(
        import(&bad)
            .unwrap_err()
            .contains("unsupported property/parameter type")
    );
    let bad = FOUNDATION.replace(
        "<general xmi:idref=\"B\"/>",
        "<general xmi:idref=\"missing\"/>",
    );
    assert!(
        import(&bad)
            .unwrap_err()
            .contains("unresolved reference missing")
    );
}

#[test]
fn duplicate_external_identifiers_fail_even_in_retained_content() {
    let bad = FOUNDATION.replace("xmi:id=\"expression\"", "xmi:id=\"C\"");
    assert!(
        import(&bad)
            .unwrap_err()
            .contains("duplicate or empty external identifier")
    );
}

#[test]
fn malformed_unknown_and_invalid_structural_input_is_explicit() {
    assert!(import("<broken>").unwrap_err().contains("malformed XML"));
    for (from, to, expected) in [
        (
            "isReadOnly=\"true\"",
            "futureFlag=\"true\"",
            "unsupported attribute futureFlag",
        ),
        (
            "isReadOnly=\"true\"",
            "isReadOnly=\"maybe\"",
            "invalid boolean",
        ),
        (
            "uml:Class",
            "uml:Activity",
            "unsupported structural input/type Activity",
        ),
        (
            "<type xmi:idref=\"B\"/>",
            "<futureElement/>",
            "unsupported child",
        ),
        ("value=\"4\"", "value=\"1\"", "lower bound exceeds"),
        ("value=\"4\"", "value=\"many\"", "invalid upper bound"),
        (
            "aggregation=\"composite\"",
            "aggregation=\"magic\"",
            "unsupported aggregation",
        ),
        (
            "<type xmi:idref=\"B\"/>",
            "<type xmi:idref=\"B\"/><type xmi:idref=\"B\"/>",
            "duplicate singleton type",
        ),
        (
            "<general xmi:idref=\"B\"/>",
            "<general xmi:idref=\"C\"/>",
            "generalization cycle",
        ),
        (
            "<type xmi:idref=\"B\"/>",
            "<type href=\"https://example.invalid/model.xmi#B\"/>",
            "unsupported property/parameter type",
        ),
    ] {
        let bad = FOUNDATION.replacen(from, to, 1);
        let error = import(&bad).unwrap_err();
        assert!(error.contains(expected), "{expected}: {error}");
    }
    let bad = FOUNDATION.replace("http://www.omg.org/spec/UML/20161101", "urn:impostor");
    assert!(import(&bad).is_err());
    let dtd = "<!DOCTYPE XMI [<!ENTITY secret SYSTEM 'file:///no-access'>]><XMI/>";
    assert!(import(dtd).unwrap_err().contains("malformed XML"));
}

#[test]
fn independent_json_cross_check_detects_intentional_mismatch_fixture() {
    let model = import(CROSS_CHECK).unwrap();
    let good = serde_json::from_str(include_str!("fixtures/cross-check.json")).unwrap();
    let bad = serde_json::from_str(include_str!("fixtures/mismatch.json")).unwrap();
    assert_eq!(
        cross_check::check(&model, &good, source(CROSS_CHECK))
            .unwrap()
            .owned_properties,
        1
    );
    assert!(
        cross_check::check(&model, &bad, source(CROSS_CHECK))
            .unwrap_err()
            .contains("Node.links")
    );
}

#[test]
fn documentary_body_keeps_text_on_both_sides_of_xml_comments() {
    let xml = CROSS_CHECK.replace(
        "</ownedAttribute>",
        "<ownedComment xmi:type=\"uml:Comment\" xmi:id=\"comment\"><body>before<!-- editorial marker -->after</body></ownedComment></ownedAttribute>",
    );
    let model = import(&xml).unwrap();
    let comment = model.properties["Node-links"]
        .retained
        .iter()
        .find(|node| node.tag == "ownedComment")
        .unwrap();
    assert_eq!(comment.children[0].text.as_deref(), Some("beforeafter"));
}
