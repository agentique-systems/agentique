use agq_metamodel_gen::{baseline, canonical_json, graph, ir::*, pipeline, sha256, xmi};
use std::{collections::BTreeMap, fs, path::PathBuf, process::Command};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn sysml_import_is_current_deterministic_and_preserves_kerml_identity() {
    let bundle = pipeline::generate_profile(&root(), baseline::SYSML).unwrap();
    let bytes = canonical_json(&bundle).unwrap();
    assert_eq!(
        bytes,
        canonical_json(&pipeline::generate_profile(&root(), baseline::SYSML).unwrap()).unwrap()
    );
    assert_eq!(
        bytes,
        fs::read(root().join(baseline::SYSML.output.unwrap())).unwrap()
    );
    let kerml = pipeline::generate(&root()).unwrap();
    assert_eq!(bundle.dependencies[baseline::KERML.id], kerml.metamodel);
    assert_eq!(bundle.representation_differences.len(), 2);
    let g = graph::combine(&bundle.metamodel, &bundle.dependencies).unwrap();
    let definition = graph::qualified(&bundle.metamodel, "Systems-DefinitionAndUsage-Definition");
    let classifier = graph::qualified(&kerml.metamodel, "Core-Classifiers-Classifier");
    assert!(
        g.classifiers[&definition]
            .generalizations
            .contains(&classifier)
    );
    assert_eq!(
        g.classifiers[&classifier]
            .entity
            .key
            .metaclass_id()
            .unwrap(),
        kerml.metamodel.classifiers["Core-Classifiers-Classifier"]
            .entity
            .key
            .metaclass_id()
            .unwrap()
    );
    let mut pending = vec![graph::qualified(
        &bundle.metamodel,
        "Systems-Parts-PartDefinition",
    )];
    let mut ancestors = std::collections::BTreeSet::new();
    while let Some(id) = pending.pop() {
        if ancestors.insert(id.clone()) {
            pending.extend(g.classifiers[&id].generalizations.clone());
        }
    }
    assert!(ancestors.contains(&classifier));
    assert!(ancestors.contains(&graph::qualified(&kerml.metamodel, "Root-Elements-Element")));
    for class in g.classifiers.values() {
        for parent in &class.generalizations {
            assert!(g.classifiers.contains_key(parent), "{parent}");
        }
    }
    let externals = xmi::external_types(&kerml.metamodel);
    for kind in [
        Kind::Class,
        Kind::Property,
        Kind::Enumeration,
        Kind::EnumerationLiteral,
        Kind::Package,
        Kind::Operation,
    ] {
        assert!(externals.values().any(|key| key.kind == kind), "{kind:?}");
    }
}

fn fixture(uri: &str, body: &str) -> String {
    format!(
        r#"<xmi:XMI xmlns:xmi="http://www.omg.org/spec/XMI/20161101" xmlns:uml="http://www.omg.org/spec/UML/20161101"><uml:Package xmi:id="root" name="SamePackage" URI="{uri}">{body}</uml:Package></xmi:XMI>"#
    )
}
fn source(xml: &str, uri: &str) -> Source {
    Source {
        specification: "Fixture".into(),
        version: "1.0".into(),
        metamodel_uri: uri.into(),
        artifact_uri: format!("{uri}/model.xmi"),
        sha256: sha256(xml.as_bytes()),
    }
}

#[test]
fn source_qualified_external_inheritance_never_joins_display_names() {
    let a = fixture(
        "urn:a",
        r#"<packagedElement xmi:type="uml:Class" xmi:id="C" name="Same"/>"#,
    );
    let a = xmi::import(&a, source(&a, "urn:a"), &BTreeMap::new()).unwrap();
    let b = fixture(
        "urn:b",
        r#"<packagedElement xmi:type="uml:Class" xmi:id="C" name="Same"><generalization xmi:type="uml:Generalization" xmi:id="g"><general href="urn:a/model.xmi#C"/></generalization></packagedElement>"#,
    );
    assert!(
        xmi::import(&b, source(&b, "urn:b"), &BTreeMap::new())
            .unwrap_err()
            .contains("unresolved referenced metaclass")
    );
    let external = xmi::external_types(&a);
    let b = xmi::import(&b, source(&b, "urn:b"), &external).unwrap();
    let g = graph::combine(&b, &BTreeMap::from([("a".into(), a.clone())])).unwrap();
    assert_eq!(g.classifiers.len(), 2);
    let ca = &g.classifiers["urn:a/model.xmi#C"].entity.key;
    let cb = &g.classifiers["urn:b/model.xmi#C"].entity.key;
    assert_ne!(ca.metaclass_id().unwrap(), cb.metaclass_id().unwrap());
    assert_eq!(ca, &a.classifiers["C"].entity.key);
    let mut renamed = b.clone();
    renamed.classifiers.get_mut("C").unwrap().entity.name = "Renamed".into();
    assert_eq!(
        b.classifiers["C"].entity.key,
        renamed.classifiers["C"].entity.key
    );
    let mut wrong = external.clone();
    wrong.get_mut("urn:a/model.xmi#C").unwrap().kind = Kind::Property;
    let bxml = fixture(
        "urn:b",
        r#"<packagedElement xmi:type="uml:Class" xmi:id="C" name="Same"><generalization xmi:type="uml:Generalization" xmi:id="g"><general href="urn:a/model.xmi#C"/></generalization></packagedElement>"#,
    );
    assert!(xmi::import(&bxml, source(&bxml, "urn:b"), &wrong).is_err());
}

#[test]
fn sysml_cli_staleness_and_wrong_baselines_fail_without_writes() {
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("sysml.json");
    let run = |check: bool| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_metamodel-gen"));
        command
            .args(["--baseline", "sysml-2.0", "--output"])
            .arg(&output);
        if check {
            command.arg("--check");
        }
        command.output().unwrap()
    };
    assert!(run(false).status.success());
    assert!(run(true).status.success());
    fs::write(&output, b"stale").unwrap();
    assert!(String::from_utf8_lossy(&run(true).stderr).contains("stale"));
    assert_eq!(fs::read(&output).unwrap(), b"stale");
    assert!(baseline::find("sysml-2.1").is_err());
    for profile in [baseline::KERML, baseline::SYSML] {
        let directory = temp
            .path()
            .join(PathBuf::from(profile.input_lock).parent().unwrap());
        fs::create_dir_all(&directory).unwrap();
        for entry in
            fs::read_dir(root().join(PathBuf::from(profile.input_lock).parent().unwrap())).unwrap()
        {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_file() {
                fs::copy(entry.path(), directory.join(entry.file_name())).unwrap();
            }
        }
    }
    let path = temp.path().join(baseline::SYSML.input_lock);
    let original: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let mut wrong = original.clone();
    wrong["version"] = "2.1".into();
    fs::write(&path, canonical_json(&wrong).unwrap()).unwrap();
    assert!(
        pipeline::generate_profile(temp.path(), baseline::SYSML)
            .unwrap_err()
            .contains("baseline")
    );
    let mut wrong = original;
    wrong["artifacts"][0]["sha256"] = "0".repeat(64).into();
    fs::write(&path, canonical_json(&wrong).unwrap()).unwrap();
    assert!(
        pipeline::generate_profile(temp.path(), baseline::SYSML)
            .unwrap_err()
            .contains("wrong authoritative artifact")
    );
}
