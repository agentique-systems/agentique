//! Exact pinned source selection for every publication slice. No semantic work.
use agq_kernel::DocumentId;
use agq_standard_libraries::VerifiedLibrarySet;
use std::collections::BTreeSet;

pub const SLICES: [&str; 5] = ["A", "B", "C", "D", "E"];
pub fn slice_documents(slice: &str) -> Option<&'static [&'static str]> {
    Some(match slice {
        "A" => &["Base.kerml", "Objects.kerml", "Links.kerml"],
        "B" => &["Occurrences.kerml", "Transfers.kerml"],
        "C" => &[
            "Performances.kerml",
            "BaseFunctions.kerml",
            "DataFunctions.kerml",
            "ControlFunctions.kerml",
        ],
        "D" => &["Observation.kerml", "FeatureReferencingPerformances.kerml"],
        "E" => &[
            "Triggers.kerml",
            "StatePerformances.kerml",
            "ControlPerformances.kerml",
            "TransitionPerformances.kerml",
        ],
        _ => return None,
    })
}
pub fn resolve_paths<'a>(
    paths: impl IntoIterator<Item = &'a str>,
    names: &[&str],
) -> Result<BTreeSet<&'a str>, String> {
    let paths: Vec<_> = paths.into_iter().collect();
    names.iter().map(|name| {
        let matches: Vec<_> = paths.iter().copied().filter(|path| {
            if name.contains('/') { path == name } else { path.rsplit('/').next() == Some(*name) }
        }).collect();
        if let [path] = matches.as_slice() { Ok(*path) } else {
            Err(format!("Slice document must identify one pinned source: {name}; exact matches: {matches:?}"))
        }
    }).collect()
}
pub fn select(
    sources: &VerifiedLibrarySet,
    names: &[&str],
) -> Result<BTreeSet<DocumentId>, String> {
    let paths = resolve_paths(sources.documents().map(|document| document.path()), names)?;
    Ok(sources
        .documents()
        .filter(|document| paths.contains(document.path()))
        .map(|document| document.document())
        .collect())
}
/// Validate all five seed lists even when resuming a subset, before any declaration
/// preparation. Slice C also uses a targeted subject from CollectionFunctions.
pub fn validate_all_documents(sources: &VerifiedLibrarySet) -> Result<(), String> {
    for slice in SLICES {
        select(sources, slice_documents(slice).expect("known slice"))?;
    }
    select(sources, &["CollectionFunctions.kerml"])?;
    Ok(())
}
