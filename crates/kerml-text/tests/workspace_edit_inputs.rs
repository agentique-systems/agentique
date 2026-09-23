//! Parser-only preflight for the held Gen2 workspace integration suite.
//! Passing this file does not establish workspace or publication acceptance.
#[path = "../../../verification/fixtures/modeling-workspace-phase1/executable_inputs.rs"]
mod inputs;

use agq_kerml_syntax::{
    DocumentId, SourceRevisionId, TextEdit,
    production::{self, Dialect, Document, SysmlSyntaxProfile},
};
use agq_kernel::provenance::ByteRange;

fn parse(source: &str, sysml: bool) -> Document {
    if sysml {
        production::parse_sysml_with_profile(
            SysmlSyntaxProfile::OperationalV2,
            DocumentId::new(),
            SourceRevisionId::new(),
            source,
            Default::default(),
        )
    } else {
        production::parse_with_dialect(
            Dialect::KerMl,
            DocumentId::new(),
            SourceRevisionId::new(),
            source,
            Default::default(),
        )
    }
    .unwrap()
}

#[test]
fn prospective_workspace_edits_parse_without_a_workspace_or_standard_replay() {
    for (source, sysml) in [
        (inputs::CONTRACTS, false),
        (inputs::REPOSITORY, true),
        (inputs::WORKSPACE, true),
    ] {
        let syntax = parse(source, sysml);
        assert!(syntax.is_complete(), "{:?}", syntax.diagnostics());
        assert_eq!(syntax.source(), source);
    }
    let initial = parse(inputs::WORKSPACE, true);
    let insertion = inputs::WORKSPACE.find("    }\n").unwrap();
    let with_port = initial
        .edit(
            &TextEdit {
                range: ByteRange::new(insertion as u64, insertion as u64).unwrap(),
                replacement: inputs::WORKSPACE_PORT.into(),
            },
            Default::default(),
        )
        .unwrap();
    assert!(with_port.is_complete());
    let insertion = with_port.source().rfind('}').unwrap();
    let specialized = with_port
        .edit(
            &TextEdit {
                range: ByteRange::new(insertion as u64, insertion as u64).unwrap(),
                replacement: inputs::WORKSPACE_SPECIALIZATION.into(),
            },
            Default::default(),
        )
        .unwrap();
    assert!(specialized.is_complete(), "{:?}", specialized.diagnostics());
    let recovered = parse(inputs::RECOVERED_REPOSITORY, true);
    assert!(!recovered.is_complete());
    assert_eq!(recovered.source(), inputs::RECOVERED_REPOSITORY);
    let missing = parse(
        &inputs::WORKSPACE.replace("Storage::Repository", "Storage::MissingRepository"),
        true,
    );
    assert!(
        missing.is_complete(),
        "unresolved names are semantic inputs"
    );
    assert!(parse("package Labels { part def 'révision'; }\n", true).is_complete());
}

#[test]
fn hundred_document_revision_inputs_use_real_frontend() {
    for index in 0..50 {
        assert!(parse(&inputs::contracts(index), false).is_complete());
        let source = inputs::worker(index);
        assert!(parse(&source, true).is_complete());
        if index == 25 {
            let with_port = inputs::with_port(&source);
            let with_specialization = inputs::with_specialization(&with_port);
            assert!(parse(&with_port, true).is_complete());
            assert!(parse(&with_specialization, true).is_complete());
        }
    }
}
