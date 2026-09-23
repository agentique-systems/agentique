//! Bounded real-source planning probe. It never invokes a producer scheduler.
use super::*;
use agq_kerml_semantics::{
    PublicationDependency, PublicationDependencyPlan, PublicationDependencyReason,
};
use agq_sysml_semantics::{
    StandardSysmlBindings, SysmlBaselineProfile, SysmlDependencyContract, SystemsLibraryIdentity,
};
use serde_json::json;
use std::collections::BTreeMap;

#[test]
#[ignore = "requires the exact accepted KerML cache and a generated report destination"]
fn systems_declared_compositional_dependency_plan() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let sources = VerifiedLibrarySet::load_from_directory(&root).unwrap();
    let cache = std::env::var_os("AGQ_ACCEPTED_KERML_CACHE").expect("accepted KerML cache path");
    let report = std::env::var_os("AGQ_COMPONENT_PLAN_REPORT").expect("generated report path");
    let accepted = CanonicalKermlStandardLibraries::restore_cache(
        std::fs::File::open(cache).unwrap(),
        &sources,
    )
    .unwrap();
    let parsed: Vec<_> = sources
        .documents()
        .filter(|source| source.language() == LibraryLanguage::SysMl)
        .map(|source| {
            let syntax = production::parse_sysml_with_profile(
                production::SysmlSyntaxProfile::OperationalV2,
                source.document(),
                source.revision(),
                source.source(),
                Default::default(),
            )
            .unwrap();
            assert!(syntax.is_complete(), "{}", source.path());
            assert_eq!(
                syntax
                    .tokens()
                    .iter()
                    .map(|token| syntax.token_text(token))
                    .collect::<String>(),
                source.source()
            );
            (source, syntax)
        })
        .collect();
    assert_eq!(parsed.len(), 21);
    let inputs: Vec<_> = parsed
        .iter()
        .map(|(source, syntax)| SourceInput {
            syntax,
            library: Some(source),
            sysml: true,
        })
        .collect();
    let base = base(&accepted).unwrap();
    let bindings = StandardSysmlBindings::unbound(SystemsLibraryIdentity::pinned(
        SystemsLibraryIdentity::SOURCE_CONTENT_SET,
    ));
    let contract = SysmlDependencyContract::checked_in_for_profile(
        &bindings,
        SysmlBaselineProfile::OperationalV2,
    )
    .unwrap();
    // Resolve only the declared graph, preserving every later producer/provider
    // obligation. This is planning input, not the previously completed candidate.
    let draft = library::refinement::refine(
        |resolved| {
            construction::construct_on(&inputs, resolved, accepted.profile(), base.clone(), None)
        },
        |draft| {
            Ok(
                systems_candidate_queries(draft, &accepted, BTreeSet::new(), &contract)?
                    .status_queries(),
            )
        },
        ReferenceRefinementStrategy::DependencyDriven,
        |round| {
            println!(
                "declared reference round={} selected={} obligations={}",
                round.round, round.selected_endpoints, round.structural_obligations
            );
        },
    )
    .unwrap();
    assert_eq!(draft.references().len(), 1327);
    let model = draft.candidate().model();
    let subjects: BTreeSet<_> = model
        .elements()
        .filter(|record| !draft.candidate().is_dependency_element(record.id()))
        .map(|record| record.id())
        .collect();
    assert_eq!(subjects.len(), 7591);
    let context = systems_candidate_context(&draft, &accepted, BTreeSet::new(), &contract).unwrap();
    let queries = systems_candidate_queries(&draft, &accepted, BTreeSet::new(), &contract).unwrap();
    let registry = systems_producer_registry(accepted.profile());
    let mut reads = Vec::new();
    let mut source_dependencies = Vec::new();
    let mut reference_status = BTreeMap::<&str, usize>::new();
    for batch in draft.references().chunks(32) {
        let q = queries.fork().status_queries();
        for reference in batch {
            let answer = q.lookup_relationship_target_with_reads(
                reference.relationship,
                reference.property,
                &reference.name,
            );
            let targets: Vec<_> = answer
                .outcome
                .value
                .iter()
                .map(|member| {
                    if reference.membership_target {
                        member.membership
                    } else {
                        member.element
                    }
                })
                .collect();
            let stored: Vec<_> = model
                .navigation_slot(reference.relationship, reference.property)
                .into_iter()
                .flat_map(|slot| slot.value().values())
                .filter_map(|value| match value {
                    Value::Reference(id) => Some(*id),
                    _ => None,
                })
                .collect();
            let status = match answer.outcome.completeness {
                Completeness::Invalid => "invalid",
                Completeness::Incomplete => "incomplete",
                Completeness::Complete if targets.is_empty() => "unresolved",
                Completeness::Complete if targets.len() != 1 => "ambiguous",
                Completeness::Complete if targets != stored => "endpoint_mismatch",
                Completeness::Complete
                    if !model
                        .registry()
                        .is_subtype(
                            model.element(targets[0]).unwrap().metaclass(),
                            reference.expected,
                        )
                        .unwrap() =>
                {
                    "endpoint_mismatch"
                }
                Completeness::Complete => "complete",
            };
            *reference_status.entry(status).or_default() += 1;
            for provider in targets {
                source_dependencies.push(PublicationDependency {
                    consumer: reference.relationship,
                    provider,
                    reason: if model
                        .registry()
                        .is_subtype(
                            model.element(reference.relationship).unwrap().metaclass(),
                            c::IMPORT,
                        )
                        .unwrap()
                    {
                        PublicationDependencyReason::SourceImport
                    } else {
                        PublicationDependencyReason::MandatoryReference
                    },
                });
            }
            reads.push((reference.relationship, answer.reads));
        }
    }
    let provider_read_rows = reads.len();
    let declared_pairs: usize = subjects
        .iter()
        .map(|subject| {
            registry
                .descriptors()
                .iter()
                .filter(|descriptor| {
                    descriptor
                        .applicability
                        .applies(model, model.element(*subject).unwrap().metaclass())
                })
                .count()
        })
        .sum();
    let plan = PublicationDependencyPlan::build(
        &context,
        &registry,
        subjects.clone(),
        source_dependencies,
        reads,
    );
    let source_paths: BTreeMap<_, _> = parsed
        .iter()
        .map(|(source, _)| (source.document(), source.path()))
        .collect();
    let components: Vec<_> = plan
        .components()
        .iter()
        .enumerate()
        .map(|(index, component)| {
            let documents: BTreeSet<_> = component
                .subjects()
                .iter()
                .map(|subject| {
                    source_paths[&draft.source_map()[&FactKey::Element(*subject)].document]
                })
                .collect();
            json!({
                "component":index,
                "subjects":component.subjects().len(),
                "documents":documents,
                "depends_on":component.dependencies(),
                "mandatory_references":draft.references().iter().filter(|reference|component.subjects().contains(&reference.relationship)).count(),
                "applicable_producer_pairs":component.subjects().iter().map(|subject| {
                    registry.descriptors().iter().filter(|descriptor| {
                        descriptor.applicability.applies(model, model.element(*subject).unwrap().metaclass())
                    }).count()
                }).sum::<usize>(),
            })
        })
        .collect();
    let output = json!({
        "format":"agq-systems-dependency-planning-audit/1",
        "boundary":"Provisional declared-graph dependency plan; no producers run, no stratum sealed, no acceptance established",
        "accepted_kerml_digest":accepted.semantic_digest(),
        "sysml_profile":SysmlBaselineProfile::OperationalV2.id(),
        "source_documents":parsed.iter().map(|(source,_)|json!({"path":source.path(),"sha256":source.sha256()})).collect::<Vec<_>>(),
        "declared_subjects":subjects.len(),
        "declared_applicable_producer_pairs":declared_pairs,
        "mandatory_references":draft.references().len(),
        "declared_reference_outcomes":reference_status,
        "construction_obligations":draft.candidate().obligations().len(),
        "reference_provider_read_rows":provider_read_rows,
        "explicit_dependency_edges":plan.dependencies().len(),
        "provider_edges":plan.dependencies().iter().filter(|edge|edge.reason == PublicationDependencyReason::ProviderRead).count(),
        "cross_component_reads":plan.dependencies().iter().filter(|edge| {
            !matches!(edge.reason, PublicationDependencyReason::Writer { .. }) &&
                matches!((plan.component_of(edge.consumer),plan.component_of(edge.provider)), (Some(a),Some(b)) if a != b)
        }).count(),
        "accepted_dependency_reads":plan.dependencies().iter().filter(|edge| {
            accepted.overlay().model().element(edge.provider).is_some()
        }).count(),
        "writer_rows":plan.writers().len(),
        "unbounded_writer_rows":plan.writers().iter().filter(|writer|writer.targets.is_none()).count(),
        "global_requirement_writer_rows":plan.writers().iter().filter(|writer|!writer.global_requirements.is_empty()).count(),
        "future_writer_rows":plan.writers().iter().filter(|writer|writer.future_subject).count(),
        "cross_component_writable_effects":plan.writers().iter().flat_map(|writer| {
            writer.targets.iter().flatten().filter(|target| {
                matches!((plan.component_of(writer.subject),plan.component_of(**target)), (Some(a),Some(b)) if a != b)
            })
        }).count(),
        "components":components,
        "diagnostics":plan.diagnostics().iter().map(|d|format!("{d:?}")).collect::<Vec<_>>(),
        "plan_digest":plan.digest(),
        "model_digest":plan.model_digest(),
        "producer_registry_digest":plan.producer_registry_digest(),
        "context_contract_digest":plan.context_contract_digest(),
    });
    assert_eq!(plan.applicable_pairs(), declared_pairs);
    std::fs::write(report, serde_json::to_vec_pretty(&output).unwrap()).unwrap();
    println!(
        "provisional components={} subjects={} pairs={declared_pairs}; no component sealed",
        plan.components().len(),
        subjects.len()
    );
    // Even an acyclic plan cannot stand in for missing producer/provider proof.
    assert!(!plan.diagnostics().is_empty());
    assert!(draft.producer_closure().is_none());
    assert!(draft.semantic_candidate().is_none());
}
