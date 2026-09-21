//! Shared observable declaration preparation for publication verification tools.
use agq_kerml::BaselineProfile;
use agq_kerml_text::library::{
    LibraryDraft, LibraryLoadError, ReferenceRefinementStrategy, refine_declarations_with_report,
};
use agq_standard_libraries::VerifiedLibrarySet;
use serde_json::{Value, json};

pub fn prepare(
    sources: &VerifiedLibrarySet,
) -> Result<(LibraryDraft, Vec<Value>), LibraryLoadError> {
    let mut rounds = Vec::new();
    let draft = refine_declarations_with_report(
        sources,
        BaselineProfile::OPERATIONAL_V8,
        ReferenceRefinementStrategy::DependencyDriven,
        |r| {
            println!(
                "refinement {}: {}/{} references evaluated, {} reused, {} endpoints, {} obligations; construction {:.3}s, context {:.3}s, diff {:.3}s, resolution {:.3}s",
                r.round,
                r.references_evaluated,
                r.references_considered,
                r.references_reused,
                r.selected_endpoints,
                r.structural_obligations,
                r.construction_elapsed.as_secs_f64(),
                r.context_elapsed.as_secs_f64(),
                r.change_detection_elapsed.as_secs_f64(),
                r.resolution_elapsed.as_secs_f64(),
            );
            rounds.push(json!({
                "round": r.round, "input_endpoints": r.input_endpoints,
                "selected_endpoints": r.selected_endpoints, "structural_obligations": r.structural_obligations,
                "references_considered": r.references_considered, "references_evaluated": r.references_evaluated,
                "references_reused": r.references_reused, "affected_elements": r.affected_elements,
                "seconds": {"construction": r.construction_elapsed.as_secs_f64(),
                    "context": r.context_elapsed.as_secs_f64(), "change_detection": r.change_detection_elapsed.as_secs_f64(),
                    "resolution": r.resolution_elapsed.as_secs_f64()},
            }));
        },
    )?;
    Ok((draft, rounds))
}
