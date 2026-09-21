//! Focused publication witnesses. This is not the full expanded-corpus audit.
use agq_kerml::{BaselineProfile, classes as c};
use agq_kerml_semantics::{
    Completeness, KerMlQueries, MemberAccess, SemanticContext, SemanticOptions,
};
use agq_kernel::{
    ElementId, Snapshot,
    provenance::{FactKey, Origin},
};
use agq_standard_libraries::VerifiedLibrarySet;
use serde_json::{Value, json};
#[path = "support/publication_metrics.rs"]
mod publication_metrics;
#[path = "support/publication_refinement.rs"]
mod publication_refinement;
use std::{collections::BTreeSet, path::Path, sync::Arc};
fn id(text: &str) -> ElementId {
    ElementId::from_u128(u128::from_str_radix(&text.replace('-', ""), 16).unwrap())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let sources = VerifiedLibrarySet::load_from_directory(&root)?;
    let (draft, refinement) = publication_refinement::prepare(&sources)?;
    let original = draft.queries(&sources)?.context().clone();
    // Ordinary atomic kernel validation; passing this is not semantic acceptance.
    let empty = Snapshot::new(Arc::new(draft.candidate().model().registry().clone()));
    let mut changes = empty.change_set();
    for record in draft.candidate().model().elements() {
        let Origin::Declared(origin) = record.origin() else {
            return Err("Unexpected source derivation".into());
        };
        changes.create(record.id(), record.metaclass(), origin.clone());
        for (property, slot) in record.slots() {
            let Origin::Declared(origin) = slot.origin() else {
                return Err("Unexpected source slot derivation".into());
            };
            changes.set(record.id(), property, slot.value().clone(), origin.clone());
        }
    }
    for link in draft.candidate().model().association_occurrences() {
        changes.link(
            link.id(),
            link.association(),
            link.ends().clone(),
            link.positions().clone(),
            link.declared_origin()
                .expect("declared library construction")
                .clone(),
        );
    }
    let snapshot = empty.apply(&changes)?;
    assert!(empty.model().is_empty());
    let context = SemanticContext::for_snapshot(
        &snapshot,
        SemanticOptions {
            baseline_profile: BaselineProfile::OPERATIONAL_V8,
            ..Default::default()
        },
        original.pinned_libraries.clone(),
    )
    .and_then(|c| c.with_available_roots((*original.available_roots).clone()))
    .map_err(|e| format!("{e:?}"))?
    .with_standard_bindings(
        draft.roots(),
        original.standard_bindings.as_ref().unwrap().library_set(),
    )
    .map_err(|e| format!("{e:?}"))?
    .with_formal_constraint_targets(
        draft.roots(),
        original.standard_bindings.as_ref().unwrap().library(),
    );
    let baseline: Value = serde_json::from_slice(&std::fs::read(
        root.join("verification/kerml-semantic-closure-v10/full-v7-complete.json"),
    )?)?;
    let references: BTreeSet<_> = baseline["reference_findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| id(r["relationship"].as_str().unwrap()))
        .collect();
    let namespaces: BTreeSet<_> = baseline["validation_findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["code"] == "validateNamespaceDistinguishibility")
        .map(|r| id(r["subject"].as_str().unwrap()))
        .collect();
    let documents: BTreeSet<_> = sources
        .documents()
        .filter(|d| {
            ["Observation.kerml", "Triggers.kerml", "Links.kerml"]
                .iter()
                .any(|p| d.path().ends_with(p))
        })
        .map(|d| d.document())
        .collect();
    let subjects: Vec<_> = draft
        .source_map()
        .iter()
        .filter_map(|(fact, source)| match fact {
            FactKey::Element(element) if documents.contains(&source.document) => Some(*element),
            _ => None,
        })
        .collect();
    // The focused regression uses the same scheduler as canonical publication.
    // Its selected population can never seal a whole-library overlay.
    let staged = true;
    let mut stages = vec![];
    let derived = agq_kerml_semantics::close_result_structure(
        &snapshot,
        agq_kerml_semantics::PublicationClosureOptions {
            initial_subjects: Some(subjects.iter().copied().collect()),
            ..Default::default()
        },
        |overlay| {
            Ok(SemanticContext::for_overlay(
                overlay,
                context.id().options.clone(),
                context.id().pinned_libraries.clone(),
            )
            .and_then(|c| c.with_available_roots((*context.id().available_roots).clone()))
            .map_err(agq_kerml_semantics::PublicationOverlayError::Context)?
            .with_standard_bindings(
                draft.roots(),
                original.standard_bindings.as_ref().unwrap().library_set(),
            )
            .map_err(agq_kerml_semantics::PublicationOverlayError::Bindings)?
            .with_formal_constraint_targets(
                draft.roots(),
                original.standard_bindings.as_ref().unwrap().library(),
            ))
        },
        |round, done, total, records| {
            if done % 256 == 0 || done == total {
                println!(
                    "Focused round {round}: {done}/{total} subjects; {records} proposed records"
                );
            }
        },
        |stage| {
            println!(
                "Focused round {} ({:?}): {} new Elements; {:?}; {:?}",
                stage.stage,
                stage.stratum,
                stage.added_elements,
                stage.completeness,
                stage.diagnostics
            );
            stages.push(json!({"stage":stage.stage,"stratum":format!("{:?}",stage.stratum),"added_elements":stage.added_elements,"complete":stage.completeness == Completeness::Complete}));
        },
    )?;
    let closed = derived.converged && derived.completeness == Completeness::Complete;
    let expanded = SemanticContext::for_overlay(
        &derived.overlay,
        context.id().options.clone(),
        context.id().pinned_libraries.clone(),
    )
    .and_then(|c| c.with_available_roots((*context.id().available_roots).clone()))
    .map_err(|e| format!("{e:?}"))?
    .with_standard_bindings(
        draft.roots(),
        original.standard_bindings.as_ref().unwrap().library_set(),
    )
    .map_err(|e| format!("{e:?}"))?
    .with_formal_constraint_targets(
        draft.roots(),
        original.standard_bindings.as_ref().unwrap().library(),
    );
    let mut rows = vec![];
    let mut failures = usize::from(staged && !closed);
    let mut capability_failures = vec![];
    if staged {
        let all_subjects: Vec<_> = subjects
            .iter()
            .copied()
            .chain(
                derived
                    .overlay
                    .model()
                    .elements()
                    .filter(|r| matches!(r.origin(), Origin::Derived(_)))
                    .map(|r| r.id()),
            )
            .collect();
        for (index, batch) in all_subjects.chunks(32).enumerate() {
            let q = KerMlQueries::new(expanded.fork());
            let audit = q.audit_publication_capabilities(batch.iter().copied());
            for (family, diagnostics) in audit.failures {
                for diagnostic in diagnostics {
                    capability_failures.push(json!({"family":format!("{family:?}"),"code":diagnostic.code,"subject":diagnostic.subject.to_string(),"message":diagnostic.message}));
                }
            }
            if index % 8 == 0 || (index + 1) * 32 >= all_subjects.len() {
                println!(
                    "Focused capability audit: {}/{} subjects; {} findings",
                    ((index + 1) * 32).min(all_subjects.len()),
                    all_subjects.len(),
                    capability_failures.len()
                );
            }
        }
        failures += capability_failures.len();
    }
    let mut namespace_rows = vec![];
    for r in draft
        .references()
        .iter()
        .filter(|r| references.contains(&r.relationship))
    {
        let q = agq_kerml_semantics::KerMlStatusQueries::new(expanded.fork());
        let answer = q.lookup_relationship_target(r.relationship, r.property, &r.name);
        let model = derived.overlay.model();
        let stored = model
            .navigation_slot(r.relationship, r.property)
            .and_then(|slot| {
                slot.value().values().find_map(|value| match value {
                    agq_kernel::value::Value::Reference(target) => Some(*target),
                    _ => None,
                })
            });
        let canonical_endpoint_valid = answer.value.as_slice().first().is_some_and(|target| {
            let target = if r.membership_target {
                target.membership
            } else {
                target.element
            };
            stored == Some(target)
                && model
                    .registry()
                    .is_subtype(model.element(target).unwrap().metaclass(), r.expected)
                    .unwrap()
        });
        failures += usize::from(
            answer.completeness != Completeness::Complete
                || answer.value.len() != 1
                || !canonical_endpoint_valid,
        );
        let row = json!({"relationship":r.relationship.to_string(),"name":r.name.segments,"completeness":format!("{:?}",answer.completeness),
            "canonical_endpoint_valid":canonical_endpoint_valid,
            "targets":answer.value.iter().map(|m|m.element.to_string()).collect::<Vec<_>>(), "diagnostics":answer.diagnostics.iter().map(|d|json!({"code":d.code,"subject":d.subject.to_string(),"message":d.message})).collect::<Vec<_>>()});
        println!("REFERENCE {row}");
        rows.push(row);
    }
    assert_eq!(
        rows.len(),
        references.len(),
        "exact historical regression population located"
    );
    for namespace in namespaces {
        let q = KerMlQueries::new(expanded.fork());
        let population = q.namespace_members(namespace, MemberAccess::All);
        let result = q.validate_namespace_distinguishability(namespace);
        failures += usize::from(result.completeness != Completeness::Complete);
        println!(
            "NAMESPACE {namespace}: {:?}; {:?}",
            result.completeness, result.diagnostics
        );
        namespace_rows.push(json!({"namespace":namespace.to_string(), "members":population.value.len(),
            "completeness":format!("{:?}",result.completeness), "findings":result.diagnostics.iter().filter(|d|d.code == "validateNamespaceDistinguishibility").count()}));
        for member in population.value {
            println!(
                "MEMBER {} {} {:?}",
                member.membership,
                member.element,
                q.effective_names(member.element).value
            );
        }
    }
    let q = KerMlQueries::new(expanded.fork());
    let cross = id("5cdfc215-1fc0-5fe1-8e1a-b5764230c704");
    let domain = q.owned_cross_feature_domain(cross);
    let featuring = q.featuring_types(cross);
    println!(
        "SELF_LINK {:?}; {:?}; {:?}",
        domain.value, featuring.value, domain.diagnostics
    );
    assert_eq!(domain.completeness, Completeness::Complete);
    assert_eq!(
        domain.value.as_ref().unwrap().owning_end,
        id("2b40cff8-b50e-5b6e-8c93-3ba39062efae")
    );
    assert_eq!(
        domain.value.as_ref().unwrap().factors[0].end,
        id("e2d92809-9575-5f03-8c8b-85028f72a22b")
    );
    assert_eq!(domain.value.as_ref().unwrap().factors.len(), 1);
    assert_eq!(featuring.completeness, Completeness::Complete);
    assert_eq!(
        featuring.value,
        domain.value.as_ref().unwrap().factors[0].types
    );
    let authority: Value = serde_json::from_slice(&std::fs::read(
        root.join("standards/kerml-1.0-operational-authority-blockers.json"),
    )?)?;
    let witnesses = &authority["blockers"][3]["complete_local_structural_facts"];
    let namespace = id(witnesses["namespace"]["id"].as_str().unwrap());
    let imported = q.imported_memberships(namespace, MemberAccess::All);
    assert_eq!(
        imported.completeness,
        Completeness::Complete,
        "{:?}",
        imported.diagnostics
    );
    let legacy_context = SemanticContext::for_snapshot(
        &snapshot,
        SemanticOptions {
            baseline_profile: BaselineProfile::OPERATIONAL_V7,
            ..Default::default()
        },
        context.id().pinned_libraries.clone(),
    )
    .map_err(|e| format!("{e:?}"))?
    .with_available_roots((*context.id().available_roots).clone())
    .map_err(|e| format!("{e:?}"))?
    .with_standard_bindings(
        draft.roots(),
        original.standard_bindings.as_ref().unwrap().library_set(),
    )
    .map_err(|e| format!("{e:?}"))?;
    let legacy =
        KerMlQueries::new(legacy_context).imported_memberships(namespace, MemberAccess::All);
    for collision in witnesses["collisions"].as_array().unwrap() {
        let removed = id(collision["imported_membership"].as_str().unwrap());
        let owned = id(collision["local_membership"].as_str().unwrap());
        assert!(!imported.value.iter().any(|m| m.membership == removed));
        assert!(legacy.value.iter().any(|m| m.membership == removed));
        assert!(
            q.namespace_members(namespace, MemberAccess::All)
                .value
                .iter()
                .any(|m| m.membership == owned)
        );
        assert!(!q.memberships_distinguishable(removed, owned).value);
        println!(
            "IMPORT_WITNESS {}: v7 retained, v8 excluded; owned {} retained",
            removed, owned
        );
    }
    // Conformance is collected independently over the focused expanded documents.
    // It neither asserts a complete overlay nor controls canonical acceptance.
    let coverage: Value = serde_json::from_slice(&std::fs::read(
        root.join("verification/kerml-publication-convergence/publication-critical-coverage.json"),
    )?)?;
    let mut report = agq_kerml_semantics::KerMlConformanceReport {
        context: expanded.id().clone(),
        diagnostics: BTreeSet::new(),
        coverage: agq_kerml_semantics::ConstraintCoverage {
            inventory: coverage["constraints"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| r["rule"].as_str().unwrap().to_owned())
                .collect(),
            checked: BTreeSet::new(),
            deferred_by_phase: BTreeSet::from(["validateElementIsImpliedIncluded".into()]),
        },
        authority_conflicts: ["KERML11-2", "KERML11-4"]
            .into_iter()
            .map(|key| {
                (
                    key.into(),
                    agq_kerml_semantics::AuthorityImpact::ValidationOnlyAuthorityConflict,
                )
            })
            .collect(),
    };
    let conformance_requested = std::env::args().any(|a| a == "--conformance");
    for batch in subjects.chunks(16).filter(|_| conformance_requested) {
        let q = KerMlQueries::new(expanded.fork());
        for &subject in batch {
            let record = derived.overlay.model().element(subject).unwrap();
            let is = |class| {
                derived
                    .overlay
                    .model()
                    .registry()
                    .is_subtype(record.metaclass(), class)
                    .unwrap()
            };
            let mut checks = vec![q.validate_local_structure(subject)];
            if is(c::NAMESPACE) {
                checks.push(q.validate_namespace_distinguishability(subject));
            }
            if is(c::FEATURE) {
                checks.push(q.validate_formal_target_constraints(subject));
            }
            for check in checks {
                report
                    .coverage
                    .checked
                    .extend(check.value.into_iter().map(str::to_owned));
                report.diagnostics.extend(check.diagnostics);
            }
        }
    }
    let conformance = json!({"format":"agentique-kerml-conformance-report/1", "scope":"FocusedDocumentPartialOverlay", "executed":conformance_requested,
        "profile":report.context.baseline_profile_id, "coverage":format!("{:?}",report.coverage.status()),
        "checked":report.coverage.checked, "deferred_by_phase":report.coverage.deferred_by_phase,
        "authority_conflicts":report.authority_conflicts.keys().map(|issue|json!({"issue":issue,"impact":"ValidationOnlyAuthorityConflict"})).collect::<Vec<_>>(),
        "diagnostics":report.diagnostics.iter().map(|d|json!({"code":d.code,"subject":d.subject.to_string(),"message":d.message})).collect::<Vec<_>>()});
    let output = std::env::args()
        .find_map(|a| a.strip_prefix("--output=").map(str::to_owned))
        .unwrap_or_else(|| {
            "verification/generated/kerml-canonical-publication/focused-closure.json".into()
        });
    let path = root.join(output);
    std::fs::create_dir_all(path.parent().unwrap())?;
    let diagnostics = derived
        .stages
        .last()
        .into_iter()
        .flat_map(|s| s.diagnostics.iter())
        .map(|d| json!({"code":d.code,"subject":d.subject.to_string(),"message":d.message}))
        .collect::<Vec<_>>();
    std::fs::write(
        path,
        serde_json::to_vec_pretty(&json!({
            "profile":BaselineProfile::OPERATIONAL_V8.id(), "verified_input_set":sources.content_set_id(),
            "reference_refinement":refinement,
            "library_set":original.standard_bindings.as_ref().unwrap().library_set().artifacts.iter().map(|(artifact, library)|
                json!({"artifact":artifact.resource(), "library":library.to_string()})).collect::<Vec<_>>(),
            "scope":"FocusedDocumentPartialOverlay", "full_expansion":false, "stages":stages,"focused_producer_closure":closed,
            "source_records":snapshot.model().elements().count(),
            "derived_records":derived.overlay.model().elements().count()-snapshot.model().elements().count(),
            "total_mandatory_references":draft.references().len(),"reference_findings":rows,
            "effective_namespaces":namespace_rows,"focused_failures":failures,"capability_failures":capability_failures,
            "self_link_domain_passed":true,"import_collision_witnesses_passed":witnesses["collisions"].as_array().unwrap().len(),
            "validated_binding_roles":original.standard_bindings.as_ref().unwrap().iter().count(),
            "counters":publication_metrics::counters(&derived.counters),
            "producer_complete":derived.completeness == Completeness::Complete, "producer_diagnostics":diagnostics,
            "conformance":conformance
        }))?,
    )?;
    if failures != 0 {
        return Err(format!("{failures} focused publication failures").into());
    }
    Ok(())
}
