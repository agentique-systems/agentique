//! Current-only enabled-control entry for the complete independent cold oracle.
//! The portable harness is also overlaid into baseline checkouts; this file is not.
#[path = "create_part_performance.rs"]
mod portable;

#[test]
#[ignore = "requires exact accepted runtime caches; never acquires or rebuilds standards"]
fn enabled_control_create_part_matches_full_self_model_reconstruction() {
    portable::run_with_command(
        |service, policy, context, command| {
            let control = agq_kerml_text::CompilationControl::new();
            assert_eq!(
                control.stage(),
                Some(agq_kerml_text::CompilationStage::Queued)
            );
            let result =
                agq_modeling_agent::propose_controlled(service, policy, context, command, &control);
            assert!(
                control.check().is_ok(),
                "the equivalence command is never cancelled"
            );
            if result.is_ok() {
                assert_eq!(
                    control.stage(),
                    Some(agq_kerml_text::CompilationStage::PreparingReview)
                );
            }
            result
        },
        "enabled_control_create_part_matches_full_self_model_reconstruction",
        "enabled-not-requested",
    );
}
