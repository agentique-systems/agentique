//! Accepted-publication tests for current Working inputs; see tests/README.md.
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

#[test]
#[ignore = "requires the accepted publication caches; never rebuilds standards"]
fn closed_malformed_attribute_typing_stays_working_until_repaired() {
    use agq_kerml_text::{SourceDiagnostic, sysml::SystemsPublicationFinding};
    use agq_kernel::provenance::Origin;
    use agq_modeling_workspace::ValidationFinding;
    use std::collections::BTreeSet;

    const SOURCE: &str =
        "package TypedTargetGate { part def NotADataType; attribute broken : NotADataType; }";
    let mut workspace = open();
    let previous = workspace.head().clone();
    assert_valid(&previous);
    let before = immutable_signature(&previous);
    let working = workspace
        .add_sysml(previous.revision(), "TypedTarget.sysml", SOURCE)
        .unwrap();
    let document = working.document_at("TypedTarget.sysml").unwrap();
    assert_eq!(document.status(), DocumentStatus::Parsed);
    assert!(working.strict_snapshot().is_some());
    let production = working.producer_status().unwrap();
    assert!(production.converged, "{production:?}");
    assert_eq!(production.completeness, Completeness::Complete);
    assert!(
        working
            .producer_closure()
            .unwrap()
            .is_fully_closed(working.semantic_model().unwrap())
    );
    assert!(working.references().iter().all(|reference| {
        reference.resolution.completeness == Completeness::Complete
            && matches!(reference.resolution.value, Resolution::Resolved(_))
    }));
    let usage = element(&working, &["TypedTargetGate", "broken"]);
    let wrong_type = element(&working, &["TypedTargetGate", "NotADataType"]);
    let queries = working.sysml_queries().unwrap();
    let direct = queries.direct_usage_types(usage);
    assert_eq!(direct.completeness(), Completeness::Complete, "{direct:?}");
    assert_eq!(direct.value(), &[wrong_type]);
    let answer = queries.effective_attribute_definitions(usage);
    assert_eq!(answer.completeness(), Completeness::Invalid, "{answer:?}");
    assert!(answer.rejected_targets.contains(&wrong_type));
    assert!(answer.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "SQ_ELEMENT_KIND" && diagnostic.subject == wrong_type
    }));
    let audit = working.effective_audit().unwrap();
    assert_eq!(audit.context(), queries.context());
    let local: BTreeSet<_> = queries
        .model()
        .elements()
        .filter(|record| {
            working
                .accepted_sysml()
                .overlay()
                .model()
                .element(record.id())
                .is_none()
        })
        .map(|record| record.id())
        .collect();
    assert_eq!(
        audit.subjects().iter().copied().collect::<BTreeSet<_>>(),
        local
    );
    assert!(local.iter().any(|subject| matches!(
        queries.model().element(*subject).unwrap().origin(),
        Origin::Derived(_)
    )));
    assert!(working.diagnostics().iter().any(|diagnostic| matches!(
        diagnostic,
        SourceDiagnostic::EffectiveAudit { origin: Some(origin), finding }
            if origin.document == document.id() && origin.revision == document.revision()
                && matches!(finding.as_ref(), SystemsPublicationFinding::Capability { diagnostic, .. }
                    if diagnostic.subject == usage
                        && diagnostic.code == "SQ_PUBLICATION_TYPED_QUERY"
                        && diagnostic.message.contains("effective attribute definitions is Invalid"))
    )));
    assert!(
        working
            .validate()
            .unwrap_err()
            .findings
            .contains(&ValidationFinding::EffectiveAudit)
    );
    assert_shared(&working);
    let signature = immutable_signature(&working);
    let rejected_context = audit.context().clone();
    let rejected_report = format!("{:?}", audit.report());
    let start = SOURCE.find("part def").unwrap();
    let repaired = workspace
        .apply(
            working.revision(),
            [edit(
                &working,
                "TypedTarget.sysml",
                start,
                start + "part".len(),
                "attribute",
            )],
        )
        .unwrap();
    assert_valid(&repaired);
    let repaired_usage = element(&repaired, &["TypedTargetGate", "broken"]);
    let repaired_type = element(&repaired, &["TypedTargetGate", "NotADataType"]);
    let repaired_queries = repaired.sysml_queries().unwrap();
    let repaired_answer = repaired_queries.effective_attribute_definitions(repaired_usage);
    assert_eq!(
        repaired_answer.completeness(),
        Completeness::Complete,
        "{repaired_answer:?}"
    );
    assert!(repaired_answer.value().contains(&repaired_type));
    let repaired_audit = repaired.effective_audit().unwrap();
    assert_eq!(repaired_audit.context(), repaired_queries.context());
    assert!(repaired_audit.report().findings.is_empty());
    assert_ne!(repaired_audit.context(), &rejected_context);
    assert_eq!(immutable_signature(&working), signature);
    assert_eq!(immutable_signature(&previous), before);
    assert_eq!(
        working.effective_audit().unwrap().context(),
        &rejected_context
    );
    assert_eq!(
        format!("{:?}", working.effective_audit().unwrap().report()),
        rejected_report
    );
    assert_eq!(
        queries
            .effective_attribute_definitions(usage)
            .completeness(),
        Completeness::Invalid
    );
    assert!(working.validate().is_err());
    assert_shared(&repaired);
}

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
    // Explicit deletion must retire the identity from the current Working
    // history even while its whole document is omitted from construction.
    let recovered_again = workspace
        .apply(
            repaired.revision(),
            [edit(
                &repaired,
                "Identity.sysml",
                start,
                start + inputs::COMPLETE_EDITED_DECLARATION.len(),
                inputs::RECOVERED_EDITED_DECLARATION,
            )],
        )
        .unwrap();
    let recovered_signature = immutable_signature(&recovered_again);
    let retained_start = recovered_again
        .document_at("Identity.sysml")
        .unwrap()
        .source()
        .find(inputs::RETAINED_DECLARATION)
        .unwrap();
    let deleted = workspace
        .apply(
            recovered_again.revision(),
            [edit(
                &recovered_again,
                "Identity.sysml",
                retained_start,
                retained_start + inputs::RETAINED_DECLARATION.len(),
                "",
            )],
        )
        .unwrap();
    let deleted_document = deleted.document_at("Identity.sysml").unwrap();
    assert_eq!(deleted_document.status(), DocumentStatus::Recovered);
    assert_eq!(deleted_document.id(), document_id);
    assert!(
        !deleted_document
            .production_syntax()
            .unwrap()
            .nodes()
            .any(|node| node.id() == node_id)
    );
    assert!(deleted.validate().is_err());
    assert_shared(&recovered_again);
    assert_shared(&deleted);
    let deleted_signature = immutable_signature(&deleted);
    let repair_start = deleted_document
        .source()
        .find(inputs::RECOVERED_EDITED_DECLARATION)
        .unwrap();
    let repaired = workspace
        .apply(
            deleted.revision(),
            [
                edit(
                    &deleted,
                    "Identity.sysml",
                    repair_start,
                    repair_start + inputs::RECOVERED_EDITED_DECLARATION.len(),
                    inputs::COMPLETE_EDITED_DECLARATION,
                ),
                edit(
                    &deleted,
                    "Identity.sysml",
                    retained_start,
                    retained_start,
                    inputs::RETAINED_DECLARATION,
                ),
            ],
        )
        .unwrap();
    assert_valid(&repaired);
    assert_eq!(
        repaired.document_at("Identity.sysml").unwrap().source(),
        inputs::IDENTITY_DOCUMENT
    );
    assert_eq!(
        repaired.document_at("Identity.sysml").unwrap().id(),
        document_id
    );
    assert_ne!(
        retained_node(repaired.document_at("Identity.sysml").unwrap()),
        node_id
    );
    let replacement = element(&repaired, &["IdentityStable", "Retained"]);
    assert_ne!(replacement, semantic_id);
    assert!(
        repaired
            .semantic_model()
            .unwrap()
            .element(semantic_id)
            .is_none()
    );
    assert_eq!(immutable_signature(&recovered_again), recovered_signature);
    assert_eq!(immutable_signature(&deleted), deleted_signature);
    let semantic_id = replacement;
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

#[test]
#[ignore = "requires the accepted publication caches; never rebuilds standards"]
fn unsupported_source_has_exact_origin_and_remains_working_until_repaired() {
    let mut workspace = open();
    let previous = workspace.head().clone();
    let before = immutable_signature(&previous);
    let working = workspace
        .add_kerml(
            previous.revision(),
            "Message.kerml",
            "package Messages { feature payload = \"unsupported\"; }",
        )
        .unwrap();
    let document = working.document_at("Message.kerml").unwrap();
    assert_eq!(document.status(), DocumentStatus::Parsed);
    assert!(working.validate().is_err());
    assert!(working.diagnostics().iter().any(|diagnostic| {
        matches!(diagnostic, agq_kerml_text::SourceDiagnostic::Unsupported { origin, construct }
            if origin.document == document.id() && origin.revision == document.revision()
                && construct == "string literal unescaping"
                && document.production_syntax().unwrap().text(origin.range).unwrap() == "\"unsupported\"")
    }));
    let answer = working.kerml_queries().unwrap().lookup_path(
        working.root(),
        &QualifiedName {
            absolute: false,
            segments: vec!["Messages".into()],
        },
    );
    assert_ne!(answer.completeness, Completeness::Complete);
    assert!(answer.value.is_empty());
    assert_shared(&working);
    let signature = immutable_signature(&working);
    let repaired = workspace
        .apply(
            working.revision(),
            [replace_by_edit(
                &working,
                "Message.kerml",
                "package Messages { feature payload; }",
            )],
        )
        .unwrap();
    assert_valid(&repaired);
    element(&repaired, &["Messages", "payload"]);
    assert_eq!(immutable_signature(&working), signature);
    assert_eq!(immutable_signature(&previous), before);
}
