use agq_kerml_text::library::corrections::OperationalLibraryPatchSet;
use agq_kernel::LibraryId;

#[test]
fn reviewed_output_identity_is_deterministic_and_content_qualified() {
    let mut patch = OperationalLibraryPatchSet::reviewed().unwrap();
    assert_eq!(patch.entries.len(), 4);
    let library = LibraryId::from_u128(11);
    let first = patch.output_id(library, "entry-a", "declaration/member");
    assert_eq!(
        first,
        patch.output_id(library, "entry-a", "declaration/member")
    );
    assert_ne!(
        first,
        patch.output_id(LibraryId::from_u128(12), "entry-a", "declaration/member")
    );
    assert_ne!(
        first,
        patch.output_id(library, "entry-b", "declaration/member")
    );
    assert_ne!(
        first,
        patch.output_id(library, "entry-a", "declaration/relationship")
    );
    patch.library_set.push_str("/changed");
    assert_ne!(
        first,
        patch.output_id(library, "entry-a", "declaration/member")
    );
    patch = OperationalLibraryPatchSet::reviewed().unwrap();
    patch.profile_id.push_str("/changed");
    assert_ne!(
        first,
        patch.output_id(library, "entry-a", "declaration/member")
    );
}

#[test]
fn validation_v4_preserves_every_v3_canonical_record_and_correction_identity() {
    use agq_kerml::BaselineProfile as P;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let sources = agq_standard_libraries::VerifiedLibrarySet::load_from_directory(&root).unwrap();
    let v3 = agq_kerml_text::library::lower_declarations_with_profile(&sources, P::OPERATIONAL_V3)
        .unwrap();
    let v4 = agq_kerml_text::library::lower_declarations_with_profile(&sources, P::OPERATIONAL_V4)
        .unwrap();
    assert_eq!(
        v3.candidate().model().elements().collect::<Vec<_>>(),
        v4.candidate().model().elements().collect::<Vec<_>>()
    );
    assert_eq!(v3.source_map(), v4.source_map());
    assert_eq!(v3.candidate().obligations(), v4.candidate().obligations());
    let q3 = v3.queries(&sources).unwrap();
    let q4 = v4.queries(&sources).unwrap();
    assert_ne!(
        q3.context().baseline_profile_id,
        q4.context().baseline_profile_id
    );
    assert_ne!(
        q3.context().errata_manifest_digest,
        q4.context().errata_manifest_digest
    );
    assert_eq!(
        q3.context().standard_bindings,
        q4.context().standard_bindings
    );
    assert_eq!(q3.context().pinned_libraries, q4.context().pinned_libraries);
}
