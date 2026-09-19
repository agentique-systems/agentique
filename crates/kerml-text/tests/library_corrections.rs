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
