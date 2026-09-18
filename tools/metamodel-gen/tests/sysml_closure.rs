use agq_metamodel_gen::{
    baseline, baseline_diagnostics, closure_audit, descriptors, graph, pipeline,
};
use std::{fs, path::PathBuf, process::Command};

#[test]
fn minimum_sysml_closure_retains_and_diagnoses_published_self_subsets() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let bundle = pipeline::generate_profile(&root, baseline::SYSML).unwrap();
    let audit = closure_audit::report(&bundle).unwrap();
    assert_eq!(audit["result"], "blocked");
    assert_eq!(audit["structural_status"], "invalid-generated-descriptor");
    assert_eq!(
        audit["registration_failure"]["kind"],
        "invalid-redefinition"
    );
    assert_eq!(
        audit["registration_failure"]["context_is_strict_subtype"],
        false
    );
    assert_eq!(
        audit["registration_failure"]["source_qualified_id"],
        "https://www.omg.org/spec/SysML/20250201/SysML.xmi#Systems-Flows-A_flowDefinition_definedFlow-definedFlow"
    );
    assert_eq!(
        audit["registration_failure"]["property_context"],
        "https://www.omg.org/spec/KerML/20250201/KerML.xmi#Kernel-Interactions-Interaction"
    );
    assert_eq!(
        audit["registration_failure"]["base_context"],
        "https://www.omg.org/spec/SysML/20250201/SysML.xmi#Systems-DefinitionAndUsage-Definition"
    );
    let diagnostics = audit["baseline_diagnostics"].as_array().unwrap();
    assert_eq!(diagnostics.len(), 3);
    for d in diagnostics {
        assert_eq!(d["disposition"], "reviewed-upstream-anomaly-preserve");
        assert_eq!(d["governing_constraint"], baseline_diagnostics::CONSTRAINT);
        assert_eq!(d["severity"], "error");
    }
    assert_eq!(audit["classifiers"]["SysML::Class"], 54);
    assert_eq!(audit["classifiers"]["KerML::Class"], 50);
    assert_eq!(audit["properties"], 562);
    assert_eq!(
        audit["same_closure_with_attributes_ports_connections"],
        true
    );
    let issues = audit["self_subsetting_properties"].as_array().unwrap();
    assert_eq!(issues.len(), 2);
    for (issue, expected) in issues.iter().zip([
        "https://www.omg.org/spec/KerML/20250201/KerML.xmi#Kernel-Associations-A_targetType_targetAssociation-targetAssociation",
        "https://www.omg.org/spec/SysML/20250201/SysML.xmi#Systems-DefinitionAndUsage-A_analysisCaseOwningUsage_nestedAnalysisCase-analysisCaseOwningUsage",
    ]) {
        assert_eq!(issue["source_qualified_id"], expected);
        assert!(issue["subsets"].as_array().unwrap().iter().any(|v| v == expected));
        assert_eq!(issue["required_by"].as_array().unwrap().last().unwrap(), expected);
    }
    let bytes = closure_audit::bytes(&bundle).unwrap();
    assert_eq!(bytes, closure_audit::bytes(&bundle).unwrap());
    assert_eq!(bytes, fs::read(root.join(closure_audit::PATH)).unwrap());
    let g = graph::combine(&bundle.metamodel, &bundle.dependencies).unwrap();
    let selected = descriptors::closure(
        &g,
        &["https://www.omg.org/spec/SysML/20250201/SysML.xmi#Systems-Parts-PartDefinition"],
    )
    .unwrap();
    // The source-qualified graph keeps the exact dependency metamodel identity.
    for id in selected.classifiers {
        let key = &g.classifiers[&id].entity.key;
        assert!(id.starts_with(&format!("{}#", key.source.artifact_uri)));
    }
}

#[test]
fn runtime_readiness_gate_fails_without_changing_outputs() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = root.join(baseline::SYSML.output.unwrap());
    let before = fs::read(&output).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_metamodel-gen"))
        .args(["--baseline", "sysml-2.0", "--require-runtime", "--check"])
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("SysML complete runtime blocked"));
    assert!(
        String::from_utf8_lossy(&result.stderr).contains("property-redefinition-context-authority")
    );
    assert!(!String::from_utf8_lossy(&result.stderr).contains("cyclic property"));
    assert_eq!(
        String::from_utf8_lossy(&result.stderr)
            .matches("subsetted-property-name")
            .count(),
        5
    );
    assert_eq!(before, fs::read(output).unwrap());
}

fn runtime_input() -> (pipeline::Bundle, descriptors::Closure) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let bundle = pipeline::generate_profile(&root, baseline::SYSML).unwrap();
    let input = closure_audit::runtime_bundle(&bundle).unwrap();
    let selected = descriptors::closure(
        &input.metamodel,
        &closure_audit::SEEDS
            .iter()
            .map(|id| graph::qualified(&bundle.metamodel, id))
            .collect::<Vec<_>>()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
    )
    .unwrap();
    (input, selected)
}

#[test]
fn exact_published_self_edges_survive_descriptor_translation_without_normalization() {
    let (input, selected) = runtime_input();
    let set = descriptors::translate_descriptors(&input, &selected).unwrap();
    let diagnostics = baseline_diagnostics::diagnose(&input.metamodel, &selected).unwrap();
    for (d, expected) in diagnostics
        .iter()
        .filter(|d| d.external_id == d.target.external_id)
        .zip([
            "c83b439d-9a37-5c2a-853f-036cbbaf5345",
            "7cff07a4-93d3-53ae-99fa-1ff751324480",
        ])
    {
        assert_eq!(d.descriptor_id, expected);
        let id = d.target.property_id().unwrap();
        let p = set.properties.iter().find(|p| p.id == id).unwrap();
        let raw =
            &input.metamodel.properties[&format!("{}#{}", d.source.artifact_uri, d.external_id)];
        assert!(p.subsets.contains(&id));
        assert_eq!(
            p.subsets,
            raw.subsets
                .iter()
                .map(|target| input.metamodel.properties[target]
                    .entity
                    .key
                    .property_id()
                    .unwrap())
                .collect()
        );
    }
    // Translation is inspection, not runtime publication. The independent
    // redefinition failure must still block the validated emitter.
    assert!(
        descriptors::descriptor_set(&input, &selected)
            .unwrap_err()
            .contains("invalid redefinition")
    );
    assert!(matches!(
        agq_kernel::metamodel::MetamodelRegistry::from_descriptors(set),
        Err(agq_kernel::metamodel::MetamodelError::InvalidRedefinition { .. })
    ));
}

#[test]
fn reviewed_dispositions_require_the_exact_source_hash_version_and_descriptor() {
    let (input, selected) = runtime_input();
    let id = "https://www.omg.org/spec/SysML/20250201/SysML.xmi#Systems-DefinitionAndUsage-A_analysisCaseOwningUsage_nestedAnalysisCase-analysisCaseOwningUsage";
    for field in 0..6 {
        let mut changed = input.metamodel.clone();
        let key = &mut changed.properties.get_mut(id).unwrap().entity.key;
        match field {
            0 => key.source.sha256 = "0".repeat(64),
            1 => key.source.artifact_uri = "https://example.org/different.xmi".into(),
            2 => key.source.version = "2.1".into(),
            3 => key.external_id.push_str("-unexpected"),
            4 => key.source.metamodel_uri.push_str("/different"),
            _ => key.package_path.push("different".into()),
        }
        let diagnostics = baseline_diagnostics::diagnose(&changed, &selected).unwrap();
        assert_eq!(
            diagnostics
                .iter()
                .filter(|d| d.disposition == "unreviewed")
                .count(),
            1
        );
    }
    let mut changed = input.metamodel.clone();
    let mut selected = selected;
    let mut unexpected = changed.properties[id].clone();
    unexpected.entity.key.external_id = "UnexpectedSelfSubset".into();
    unexpected.entity.key.source.artifact_uri = "https://example.org/new.xmi".into();
    let key = "https://example.org/new.xmi#UnexpectedSelfSubset".to_owned();
    unexpected.subsets = vec![key.clone()];
    changed.properties.insert(key.clone(), unexpected);
    selected.properties.insert(key);
    let diagnostics = baseline_diagnostics::diagnose(&changed, &selected).unwrap();
    assert_eq!(diagnostics.len(), 4);
    assert_eq!(
        diagnostics
            .iter()
            .filter(|d| d.disposition == "unreviewed")
            .count(),
        1
    );
}

#[test]
fn explicit_sysml_check_checks_the_audit_and_rejects_stale_audit_without_writing() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let directory = tempfile::tempdir().unwrap();
    let audit_path = directory.path().join(closure_audit::PATH);
    fs::create_dir_all(audit_path.parent().unwrap()).unwrap();
    fs::write(&audit_path, b"stale").unwrap();
    for file in ["full-audit.json", "full.golden.json"] {
        let path = PathBuf::from("standards/generated/sysml-2.0").join(file);
        fs::copy(root.join(&path), directory.path().join(path)).unwrap();
    }
    let result = Command::new(env!("CARGO_BIN_EXE_metamodel-gen"))
        .args(["--baseline", "sysml-2.0", "--check", "--root"])
        .arg(root)
        .arg("--descriptor-output")
        .arg(directory.path())
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("generated output is stale"));
    assert_eq!(fs::read(audit_path).unwrap(), b"stale");
}

#[test]
fn kerml_association_closure_registers_with_its_exact_published_self_subset() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let bundle = pipeline::generate(&root).unwrap();
    let selected =
        descriptors::closure(&bundle.metamodel, &["Kernel-Associations-Association"]).unwrap();
    let set = descriptors::descriptor_set(&bundle, &selected).unwrap();
    let registry = agq_kernel::metamodel::MetamodelRegistry::from_descriptors(set).unwrap();
    let p = bundle.metamodel.properties["Kernel-Associations-A_targetType_targetAssociation-targetAssociation"].entity.key.property_id().unwrap();
    assert!(registry.property(p).unwrap().subsets.contains(&p));
    assert!(registry.subset_closure(p).unwrap().contains(&p));
    assert!(registry.subset_contributors(p).unwrap().contains(&p));
    assert!(registry.subset_contributors(p).unwrap().len() <= selected.properties.len());
}
