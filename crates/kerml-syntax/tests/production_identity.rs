use agq_kerml_syntax::{DocumentId, SourceRevisionId, production};

#[test]
fn restored_arena_authenticates_shape_and_unique_identity() {
    let document = DocumentId::new();
    let revision = SourceRevisionId::new();
    let parse = || {
        production::parse(
            document,
            revision,
            "namespace P { type A; type B; }",
            Default::default(),
        )
        .unwrap()
    };
    let original = parse();
    let checkpoint = original.identity_checkpoint();
    let restored = parse().restore_identities(&checkpoint).unwrap();
    assert_eq!(restored.identity_checkpoint(), checkpoint);
    assert_eq!(restored.source(), original.source());
    let mut changed = checkpoint.clone();
    changed[0].production.push('X');
    assert!(parse().restore_identities(&changed).is_err());
    let mut duplicate = checkpoint.clone();
    duplicate[1].id = duplicate[0].id;
    assert!(parse().restore_identities(&duplicate).is_err());
    assert!(parse().restore_identities(&checkpoint[1..]).is_err());
}
