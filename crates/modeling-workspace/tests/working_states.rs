//! Held acceptance tests for Working inputs. No manifest or production workspace
//! exists yet; see the frontend-boundary design and tests/README.md.
#[path = "../../../verification/fixtures/modeling-workspace-phase1/working_state_inputs.rs"]
mod inputs;
#[allow(dead_code)]
mod support;

use agq_kerml_semantics::{Completeness, QualifiedName, Resolution};
use agq_kerml_syntax::{SyntaxNodeId, production::Production};
use agq_kerml_text::{DocumentStatus, ProjectChange, ProjectDocument};
use agq_kernel::value::Value;
use agq_sysml_semantics::PendingSysmlRule;
use std::sync::Arc;
use support::*;

fn retained_node(document: &ProjectDocument) -> SyntaxNodeId {
    document
        .production_syntax()
        .unwrap()
        .nodes()
        .find(|node| {
            node.kind() == Production::PartDefinition && node.text() == inputs::RETAINED_DECLARATION
        })
        .expect("disjoint complete declaration is retained by syntax reconciliation")
        .id()
}

#[test]
#[ignore = "requires the accepted publication caches; never rebuilds standards"]
fn temporary_recovery_preserves_reconciled_identity_until_explicit_removal() {
    let mut workspace = open();
    let seed = seed(&mut workspace);
    let valid = workspace
        .add_sysml(seed.revision(), "Identity.sysml", inputs::IDENTITY_DOCUMENT)
        .unwrap();
    assert_valid(&valid);
    let old = valid.validate().unwrap();
    let original_signature = immutable_signature(&valid);
    let document = valid.document_at("Identity.sysml").unwrap();
    let document_id = document.id();
    let node_id = retained_node(document);
    let semantic_id = element(&valid, &["IdentityStable", "Retained"]);
    let start = document
        .source()
        .find(inputs::COMPLETE_EDITED_DECLARATION)
        .unwrap();
    let recovered = workspace
        .apply(
            valid.revision(),
            [edit(
                &valid,
                "Identity.sysml",
                start,
                start + inputs::COMPLETE_EDITED_DECLARATION.len(),
                inputs::RECOVERED_EDITED_DECLARATION,
            )],
        )
        .unwrap();
    assert!(Arc::ptr_eq(workspace.head(), &recovered));
    assert_eq!(recovered.parent(), Some(valid.revision()));
    assert_ne!(recovered.revision(), valid.revision());
    assert!(recovered.validate().is_err());
    assert!(!recovered.diagnostics().is_empty());
    let current = recovered.document_at("Identity.sysml").unwrap();
    assert_eq!(current.id(), document_id);
    assert_ne!(current.revision(), document.revision());
    assert_eq!(current.source(), inputs::recovered_identity_document());
    assert_eq!(current.status(), DocumentStatus::Recovered);
    assert_eq!(retained_node(current), node_id);
    assert!(
        recovered
            .semantic_model()
            .is_none_or(|model| model.element(semantic_id).is_none()),
        "initial safe lowering omits the whole recovered document"
    );
    if let Ok(q) = recovered.kerml_queries() {
        let lookup = q.lookup_path(
            recovered.root(),
            &QualifiedName {
                absolute: false,
                segments: vec!["IdentityStable".into(), "Retained".into()],
            },
        );
        assert_ne!(lookup.completeness, Completeness::Complete);
        assert!(
            lookup
                .value
                .iter()
                .all(|member| member.element != semantic_id)
        );
    }
    assert_eq!(immutable_signature(old.working()), original_signature);
    assert_shared(&recovered);
    let repaired = workspace
        .apply(
            recovered.revision(),
            [edit(
                &recovered,
                "Identity.sysml",
                start,
                start + inputs::RECOVERED_EDITED_DECLARATION.len(),
                inputs::COMPLETE_EDITED_DECLARATION,
            )],
        )
        .unwrap();
    assert_valid(&repaired);
    let repaired_document = repaired.document_at("Identity.sysml").unwrap();
    assert_eq!(repaired_document.id(), document_id);
    assert_eq!(repaired_document.source(), inputs::IDENTITY_DOCUMENT);
    assert_eq!(retained_node(repaired_document), node_id);
    assert_eq!(
        element(&repaired, &["IdentityStable", "Retained"]),
        semantic_id
    );
    assert!(std::ptr::eq(
        valid.document_at("Contracts.kerml").unwrap(),
        repaired.document_at("Contracts.kerml").unwrap()
    ));
    let removed = workspace
        .apply(
            repaired.revision(),
            [ProjectChange::Remove {
                document: document_id,
            }],
        )
        .unwrap();
    assert!(removed.document_at("Identity.sysml").is_none());
    assert!(
        removed
            .semantic_model()
            .unwrap()
            .element(semantic_id)
            .is_none()
    );
    let readded = workspace
        .add_sysml(
            removed.revision(),
            "Identity.sysml",
            inputs::IDENTITY_DOCUMENT,
        )
        .unwrap();
    assert_valid(&readded);
    let new_document = readded.document_at("Identity.sysml").unwrap();
    assert_ne!(new_document.id(), document_id);
    assert_ne!(retained_node(new_document), node_id);
    assert_ne!(
        element(&readded, &["IdentityStable", "Retained"]),
        semantic_id
    );
    assert_eq!(immutable_signature(&valid), original_signature);
}

#[test]
#[ignore = "requires the accepted publication caches; never rebuilds standards"]
fn unresolved_reference_then_provider_removal_and_repair_never_resurrects_old_endpoint() {
    let mut workspace = open();
    let valid = seed(&mut workspace);
    assert_valid(&valid);
    let old_signature = immutable_signature(&valid);
    let provider = element(&valid, &["Storage", "Repository"]);
    let document = valid.document_at("Workspace.sysml").unwrap();
    let start = document.source().find("Storage::Repository").unwrap();
    let unresolved = workspace
        .apply(
            valid.revision(),
            [edit(
                &valid,
                "Workspace.sysml",
                start,
                start + "Storage::Repository".len(),
                "Storage::MissingRepository",
            )],
        )
        .unwrap();
    assert_unresolved(&unresolved, None);
    assert!(Arc::ptr_eq(workspace.head(), &unresolved));
    let current = unresolved.document_at("Workspace.sysml").unwrap();
    assert_eq!(current.status(), DocumentStatus::Parsed);
    assert_eq!(current.id(), document.id());
    assert_ne!(current.revision(), document.revision());
    let reference = unresolved
        .references()
        .iter()
        .find(|reference| reference.name.segments == ["Storage", "MissingRepository"])
        .expect("the failed mandatory reference is retained, not dropped with its edge");
    assert_eq!(reference.origin.document, current.id());
    assert_eq!(reference.origin.revision, current.revision());
    assert!(
        current
            .production_syntax()
            .unwrap()
            .text(reference.origin.range)
            .unwrap()
            .contains("MissingRepository")
    );
    assert!(!matches!(
        reference.resolution.value,
        Resolution::Resolved(_)
    ));
    if let Ok(q) = unresolved.kerml_queries() {
        assert_eq!(&reference.resolution.context, q.context());
        assert!(
            q.model()
                .navigation_slot(
                    reference.relationship,
                    agq_kerml::properties::FEATURE_TYPING_TYPE
                )
                .is_none_or(|slot| slot
                    .value()
                    .values()
                    .all(|value| value != &Value::Reference(provider))),
            "an unresolved reference cannot retain its old canonical endpoint"
        );
        if q.model().element(reference.specific).is_some() {
            assert!(
                !q.direct_feature_types(reference.specific)
                    .value
                    .contains(&provider)
            );
            let effective = unresolved
                .sysml_queries()
                .unwrap()
                .effective_part_definitions(reference.specific);
            assert_ne!(effective.completeness(), Completeness::Complete);
        }
    }
    assert_eq!(immutable_signature(&valid), old_signature);
    assert_eq!(element(&valid, &["Storage", "Repository"]), provider);

    // Retirement must advance from the current Working inputs, not from the
    // last validated revision. The consumer stays unresolved across removal.
    let unresolved_signature = immutable_signature(&unresolved);
    let provider_document = valid.document_at("Repository.sysml").unwrap();
    let removed = workspace
        .apply(
            unresolved.revision(),
            [ProjectChange::Remove {
                document: provider_document.id(),
            }],
        )
        .unwrap();
    assert_eq!(removed.parent(), Some(unresolved.revision()));
    assert!(removed.document_at("Repository.sysml").is_none());
    assert_unresolved(&removed, Some(provider));
    assert!(std::ptr::eq(
        removed.document_at("Workspace.sysml").unwrap(),
        current
    ));
    let removed_reference = removed
        .references()
        .iter()
        .find(|reference| reference.name.segments == ["Storage", "MissingRepository"])
        .expect("Working-to-Working removal retains the exact mandatory failure");
    assert_eq!(removed_reference.origin.document, current.id());
    assert_eq!(removed_reference.origin.revision, current.revision());
    assert!(!matches!(
        removed_reference.resolution.value,
        Resolution::Resolved(_)
    ));
    if let Ok(q) = removed.kerml_queries() {
        assert_eq!(&removed_reference.resolution.context, q.context());
        assert!(
            !q.direct_feature_types(removed_reference.specific)
                .value
                .contains(&provider)
        );
    }
    let removed_signature = immutable_signature(&removed);

    // Restoring the original name while its provider is absent must still
    // publish Working with current source evidence and no historical endpoint.
    let waiting = workspace
        .apply(
            removed.revision(),
            [edit(
                &removed,
                "Workspace.sysml",
                start,
                start + "Storage::MissingRepository".len(),
                "Storage::Repository",
            )],
        )
        .unwrap();
    assert_eq!(waiting.parent(), Some(removed.revision()));
    assert!(Arc::ptr_eq(workspace.head(), &waiting));
    assert_unresolved(&waiting, Some(provider));
    let waiting_document = waiting.document_at("Workspace.sysml").unwrap();
    assert_eq!(waiting_document.source(), document.source());
    assert_eq!(waiting_document.id(), document.id());
    assert_ne!(waiting_document.revision(), current.revision());
    let waiting_reference = waiting
        .references()
        .iter()
        .find(|reference| reference.name.segments == ["Storage", "Repository"])
        .expect("repairing only the name retains the still-missing mandatory reference");
    assert_eq!(waiting_reference.origin.document, waiting_document.id());
    assert_eq!(
        waiting_reference.origin.revision,
        waiting_document.revision()
    );
    assert!(
        waiting_document
            .production_syntax()
            .unwrap()
            .text(waiting_reference.origin.range)
            .unwrap()
            .contains("Storage::Repository")
    );
    assert!(!matches!(
        waiting_reference.resolution.value,
        Resolution::Resolved(_)
    ));
    if let Ok(q) = waiting.kerml_queries() {
        assert_eq!(&waiting_reference.resolution.context, q.context());
        assert!(
            !q.direct_feature_types(waiting_reference.specific)
                .value
                .contains(&provider)
        );
        assert!(
            q.model()
                .navigation_slot(
                    waiting_reference.relationship,
                    agq_kerml::properties::FEATURE_TYPING_TYPE
                )
                .is_none_or(|slot| slot
                    .value()
                    .values()
                    .all(|value| value != &Value::Reference(provider)))
        );
    }
    let waiting_signature = immutable_signature(&waiting);
    let repaired = workspace
        .add_sysml(
            waiting.revision(),
            "Repository.sysml",
            provider_document.source(),
        )
        .unwrap();
    assert_valid(&repaired);
    let new_provider = element(&repaired, &["Storage", "Repository"]);
    assert_ne!(new_provider, provider);
    assert_ne!(
        repaired.document_at("Repository.sysml").unwrap().id(),
        provider_document.id()
    );
    assert!(
        repaired
            .semantic_model()
            .unwrap()
            .element(provider)
            .is_none()
    );
    assert!(std::ptr::eq(
        repaired.document_at("Workspace.sysml").unwrap(),
        waiting_document
    ));
    let repaired_reference = repaired
        .references()
        .iter()
        .find(|reference| reference.name.segments == ["Storage", "Repository"])
        .unwrap();
    assert_eq!(
        repaired_reference.resolution.value,
        Resolution::Resolved(new_provider)
    );
    let q = repaired.kerml_queries().unwrap();
    assert_eq!(&repaired_reference.resolution.context, q.context());
    let types = q.direct_feature_types(repaired_reference.specific);
    assert_eq!(types.completeness, Completeness::Complete);
    assert!(types.value.contains(&new_provider));
    assert!(!types.value.contains(&provider));
    assert_eq!(immutable_signature(&valid), old_signature);
    assert_eq!(immutable_signature(&unresolved), unresolved_signature);
    assert_eq!(immutable_signature(&removed), removed_signature);
    assert_eq!(immutable_signature(&waiting), waiting_signature);
}

#[test]
#[ignore = "requires the accepted publication caches; never rebuilds standards"]
fn parsed_unsupported_variation_cannot_validate_even_when_producers_finish() {
    let mut workspace = open();
    let base = seed(&mut workspace);
    let working = workspace
        .add_sysml(base.revision(), "Choices.sysml", inputs::VARIATION_DOCUMENT)
        .unwrap();
    let document = working.document_at("Choices.sysml").unwrap();
    assert_eq!(document.source(), inputs::VARIATION_DOCUMENT);
    assert_eq!(document.status(), DocumentStatus::Parsed);
    let status = working
        .producer_status()
        .expect("actual local producer run");
    assert!(status.converged, "{status:?}");
    assert_eq!(status.completeness, Completeness::Complete, "{status:?}");
    assert!(
        working
            .producer_closure()
            .expect("complete run has a certificate")
            .is_fully_closed(working.semantic_model().unwrap())
    );
    assert!(working.validate().is_err());
    assert!(!working.diagnostics().is_empty());
    let owner = element(&working, &["Choices", "Configurable"]);
    let usage = element(&working, &["Choices", "Configurable", "selected"]);
    let result = working.sysml_queries().unwrap().effective_usages(owner);
    assert!(result.value().contains(&usage));
    assert_eq!(result.completeness(), Completeness::Incomplete);
    assert!(
        result
            .pending
            .contains(&(usage, PendingSysmlRule::Variation))
    );
    // Validation cannot infer supported semantics from scheduler convergence or
    // certificate coverage. The lower-level variation regression closes both.
    let signature = immutable_signature(&working);
    let start = document.source().find("variation ").unwrap();
    let repaired = workspace
        .apply(
            working.revision(),
            [edit(
                &working,
                "Choices.sysml",
                start,
                start + "variation ".len(),
                "",
            )],
        )
        .unwrap();
    assert_valid(&repaired);
    assert_eq!(
        repaired.document_at("Choices.sysml").unwrap().source(),
        inputs::ORDINARY_DOCUMENT
    );
    assert_eq!(immutable_signature(&working), signature);
    assert_eq!(
        working
            .sysml_queries()
            .unwrap()
            .effective_usages(owner)
            .completeness(),
        Completeness::Incomplete
    );
}
