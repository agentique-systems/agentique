//! Parser and edit-identity preflight only. No workspace or accepted graph load.
#[path = "../../../verification/fixtures/modeling-workspace-phase1/working_state_inputs.rs"]
mod inputs;

use agq_kerml_syntax::{
    DocumentId, SourceRevisionId, SyntaxNodeId, TextEdit,
    production::{self, Document, Production, SysmlSyntaxProfile},
};
use agq_kernel::provenance::ByteRange;

fn parse(source: &str) -> Document {
    production::parse_sysml_with_profile(
        SysmlSyntaxProfile::OperationalV2,
        DocumentId::new(),
        SourceRevisionId::new(),
        source,
        Default::default(),
    )
    .unwrap()
}

fn retained(syntax: &Document) -> SyntaxNodeId {
    syntax
        .nodes()
        .find(|node| {
            node.kind() == Production::PartDefinition && node.text() == inputs::RETAINED_DECLARATION
        })
        .expect("complete disjoint declaration remains in the syntax arena")
        .id()
}

#[test]
fn working_recovery_retains_disjoint_syntax_identity_but_readding_does_not() {
    let original = parse(inputs::IDENTITY_DOCUMENT);
    assert!(original.is_complete(), "{:?}", original.diagnostics());
    let retained_id = retained(&original);
    let start = original
        .source()
        .find(inputs::COMPLETE_EDITED_DECLARATION)
        .unwrap();
    let recovered = original
        .edit(
            &TextEdit {
                range: ByteRange::new(
                    start as u64,
                    (start + inputs::COMPLETE_EDITED_DECLARATION.len()) as u64,
                )
                .unwrap(),
                replacement: inputs::RECOVERED_EDITED_DECLARATION.into(),
            },
            Default::default(),
        )
        .unwrap();
    assert!(!recovered.is_complete());
    assert_eq!(recovered.source(), inputs::recovered_identity_document());
    assert_eq!(recovered.document(), original.document());
    assert_ne!(recovered.revision(), original.revision());
    assert_eq!(retained(&recovered), retained_id);
    let repaired = recovered
        .edit(
            &TextEdit {
                range: ByteRange::new(
                    start as u64,
                    (start + inputs::RECOVERED_EDITED_DECLARATION.len()) as u64,
                )
                .unwrap(),
                replacement: inputs::COMPLETE_EDITED_DECLARATION.into(),
            },
            Default::default(),
        )
        .unwrap();
    assert!(repaired.is_complete(), "{:?}", repaired.diagnostics());
    assert_eq!(repaired.source(), inputs::IDENTITY_DOCUMENT);
    assert_eq!(repaired.document(), original.document());
    assert_ne!(repaired.revision(), recovered.revision());
    assert_eq!(retained(&repaired), retained_id);
    assert_eq!(original.source(), inputs::IDENTITY_DOCUMENT);
    assert!(original.is_complete());
    let readded = parse(inputs::IDENTITY_DOCUMENT);
    assert_ne!(readded.document(), original.document());
    assert_ne!(retained(&readded), retained_id);
}

#[test]
fn working_missing_targets_and_unsupported_variation_are_complete_syntax() {
    for source in [
        inputs::VARIATION_DOCUMENT,
        inputs::ORDINARY_DOCUMENT,
        "package Modeling { part def Workspace { part repository : Storage::MissingRepository; } }",
    ] {
        let syntax = parse(source);
        assert!(syntax.is_complete(), "{source}: {:?}", syntax.diagnostics());
        assert_eq!(syntax.source(), source);
        assert!(syntax.recovery().is_empty());
        assert_eq!(
            syntax
                .tokens()
                .iter()
                .map(|token| syntax.token_text(token))
                .collect::<String>(),
            source
        );
    }
}
