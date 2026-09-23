//! Check planned workspace inputs with the real frontend. These tests do not
//! instantiate a workspace, consume publications or assert semantic acceptance.
use agq_kerml_syntax::{
    DocumentId, SourceRevisionId,
    production::{self, Dialect, Document, SysmlSyntaxProfile},
};
use serde_json::Value;

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../verification/fixtures/modeling-workspace-phase1/working-revisions.json"
    ))
    .unwrap()
}

fn parse(language: &str, source: &str) -> Document {
    match language {
        "SysML" => production::parse_sysml_with_profile(
            SysmlSyntaxProfile::OperationalV2,
            DocumentId::new(),
            SourceRevisionId::new(),
            source,
            Default::default(),
        ),
        "KerML" => production::parse_with_dialect(
            Dialect::KerMl,
            DocumentId::new(),
            SourceRevisionId::new(),
            source,
            Default::default(),
        ),
        _ => panic!("unknown fixture language"),
    }
    .unwrap()
}

#[test]
fn planned_working_inputs_separate_recovery_from_unresolved_source_names() {
    let fixture = fixture();
    for document in fixture["initial_documents"].as_array().unwrap() {
        let source = document["source"].as_str().unwrap();
        let syntax = parse(document["language"].as_str().unwrap(), source);
        assert!(syntax.is_complete(), "{}", document["path"]);
        assert_eq!(syntax.source(), source);
    }
    for scenario in fixture["independent_scenarios"].as_array().unwrap() {
        let source = scenario["source"].as_str().unwrap();
        let syntax = parse("SysML", source);
        assert_eq!(syntax.source(), source);
        assert_eq!(syntax.is_complete(), scenario["syntax"] == "Parsed");
        if scenario["syntax"] == "Recovered" {
            assert!(!syntax.recovery().is_empty() || !syntax.diagnostics().is_empty());
        }
    }
}

#[test]
fn hundred_document_scaling_inputs_parse_with_explicit_languages() {
    let fixture = fixture();
    let scaling = &fixture["scaling"];
    let mut paths = std::collections::BTreeSet::new();
    let mut languages = std::collections::BTreeMap::new();
    for index in 0..scaling["groups"].as_u64().unwrap() {
        for template in scaling["templates"].as_array().unwrap() {
            let source = template["source"]
                .as_str()
                .unwrap()
                .replace("$INDEX", &format!("{index:03}"));
            let path = template["path"]
                .as_str()
                .unwrap()
                .replace("$INDEX", &format!("{index:03}"));
            assert!(paths.insert(path.clone()));
            let language = template["language"].as_str().unwrap();
            *languages.entry(language).or_insert(0) += 1;
            let syntax = parse(language, &source);
            assert!(syntax.is_complete(), "{path}: {:?}", syntax.diagnostics());
            assert_eq!(syntax.source(), source);
        }
    }
    assert_eq!(paths.len(), 100);
    assert_eq!(
        languages,
        std::collections::BTreeMap::from([("KerML", 50), ("SysML", 50)])
    );
}
