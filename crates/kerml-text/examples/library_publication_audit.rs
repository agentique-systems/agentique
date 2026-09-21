//! Complete corpus storage/producer/implemented-validation audit, without acceptance.
use agq_kerml::{BaselineProfile, classes as c};
use agq_kerml_semantics::{
    Completeness, Diagnostic, KerMlQueries, QueryResult, SemanticContext, SemanticOptions,
};
use agq_kernel::{Snapshot, provenance::Origin};
use agq_standard_libraries::VerifiedLibrarySet;
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Write,
    path::Path,
    sync::Arc,
};

#[derive(Default)]
struct Checks {
    count: usize,
    incomplete: usize,
    invalid: usize,
    diagnostics: BTreeSet<Diagnostic>,
    rules: BTreeMap<&'static str, usize>,
}
impl Checks {
    fn query<T>(&mut self, answer: QueryResult<T>) {
        self.count += 1;
        self.incomplete += usize::from(answer.completeness == Completeness::Incomplete);
        self.invalid += usize::from(answer.completeness == Completeness::Invalid);
        self.diagnostics.extend(answer.diagnostics);
    }
    fn validation(&mut self, answer: QueryResult<Vec<&'static str>>) {
        for &rule in &answer.value {
            *self.rules.entry(rule).or_default() += 1;
        }
        self.query(answer);
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source_only = std::env::args().any(|a| a == "--source-only");
    let path = std::env::args()
        .find_map(|a| a.strip_prefix("--output=").map(str::to_owned))
        .ok_or("--output is required")?;
    if Path::new(&path).exists() {
        return Err("Audit output already exists; use a new evidence path".into());
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let sources = VerifiedLibrarySet::load_from_directory(&root)?;
    let draft = agq_kerml_text::library::refine_declarations_with_profile(
        &sources,
        BaselineProfile::OPERATIONAL_V7,
        |round, refs, obligations| {
            println!("refinement {round}: {refs} endpoints, {obligations} obligations")
        },
    )?;
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
            link.origin().clone(),
        );
    }
    let snapshot = empty.apply(&changes)?;
    assert!(empty.model().is_empty());
    let context = SemanticContext::for_snapshot(
        &snapshot,
        SemanticOptions {
            baseline_profile: BaselineProfile::OPERATIONAL_V7,
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
    let mut references = Checks::default();
    let mut unresolved = 0;
    let mut ambiguous = 0;
    let mut ref_rows = vec![];
    for batch in draft.references().chunks(128) {
        let q = KerMlQueries::new(context.fork());
        for r in batch {
            let answer = q.lookup_relationship_target(r.relationship, r.property, &r.name);
            unresolved += usize::from(answer.value.is_empty());
            ambiguous += usize::from(answer.value.len() > 1);
            if answer.completeness != Completeness::Complete || answer.value.len() != 1 {
                ref_rows.push(json!({"relationship":r.relationship.to_string(),"name":r.name.segments,"completeness":format!("{:?}",answer.completeness),
                    "diagnostics":answer.diagnostics.iter().map(|d| json!({"code":d.code,"subject":d.subject.to_string(),"message":d.message})).collect::<Vec<_>>() }));
            }
            references.query(answer);
        }
    }
    println!(
        "references: {unresolved} unresolved, {} incomplete, {ambiguous} ambiguous",
        references.incomplete
    );
    let ids: Vec<_> = snapshot.model().elements().map(|r| r.id()).collect();
    let q = KerMlQueries::new(context.fork());
    let mut plan = q.plan_result_structure([]);
    for (index, batch) in ids.chunks(128).enumerate().filter(|_| !source_only) {
        let q = KerMlQueries::new(context.fork());
        plan.merge(q.plan_result_structure(batch.iter().copied()))?;
        if index % 25 == 0 {
            println!(
                "producer batch {index}: {} proposed records",
                plan.planned_elements().count()
            );
        }
    }
    let planned = plan.planned_elements().count();
    let producer_complete = plan.production.completeness == Completeness::Complete;
    let producer_diagnostics: Vec<_> = plan
        .production
        .diagnostics
        .iter()
        .map(|d| json!({"code":d.code,"subject":d.subject.to_string(),"message":d.message}))
        .collect();
    let produced = plan.materialize(&snapshot);
    let mut checks = Checks::default();
    let mut expanded_references = Checks::default();
    let mut expanded_unresolved = 0;
    let mut expanded_ambiguous = 0;
    let mut expanded_ref_rows = vec![];
    let mut witness_rows = vec![];
    let (derived_count, overlay_error) = match produced {
        Ok(result) => {
            // The overlay retains canonical provenance. The aggregate producer
            // proof is already summarized above and need not coexist with every
            // validation query's own recursively expanded evidence.
            let agq_kerml_semantics::ResultStructure {
                overlay,
                production,
                contextual_results,
            } = result;
            drop(production);
            drop(contextual_results);
            let derived_count = overlay.model().elements().count() - ids.len();
            println!(
                "Materialized {derived_count} derived records; released aggregate producer proof"
            );
            let derived_context = SemanticContext::for_overlay(
                &overlay,
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
            println!("Expanded semantic context ready");
            let selector: serde_json::Value = serde_json::from_slice(&std::fs::read(
                root.join("verification/kerml-semantic-closure-v10/selector-verification.json"),
            )?)?;
            for witness in selector["witnesses"].as_array().unwrap() {
                let feature = agq_kernel::ElementId::from_u128(u128::from_str_radix(
                    &witness["feature"].as_str().unwrap().replace('-', ""),
                    16,
                )?);
                let q = KerMlQueries::new(derived_context.fork());
                let answer = q.owned_cross_feature(feature);
                assert_eq!(answer.completeness, Completeness::Complete);
                assert_eq!(answer.value, None, "expanded Occurrences witness");
                println!(
                    "Expanded Occurrences witness {feature}: Complete, no owned cross Feature"
                );
                let sequence = q.memberships(feature).value.into_iter().map(|membership| {
                    let member = q.member(membership).value.unwrap();
                    json!({"membership":membership.to_string(),"member":member.to_string(),
                        "membership_metaclass":overlay.model().registry().class(overlay.model().element(membership).unwrap().metaclass()).unwrap().name,
                        "member_metaclass":overlay.model().registry().class(overlay.model().element(member).unwrap().metaclass()).unwrap().name})
                }).collect::<Vec<_>>();
                witness_rows.push(json!({"path":witness["path"],"feature":feature.to_string(),"selected":null,"completeness":"Complete","owned_membership_sequence":sequence}));
            }
            for (index, batch) in draft
                .references()
                .chunks(16)
                .enumerate()
                .filter(|_| !source_only)
            {
                let q = KerMlQueries::new(derived_context.fork());
                for r in batch {
                    let answer = q.lookup_relationship_target(r.relationship, r.property, &r.name);
                    expanded_unresolved += usize::from(answer.value.is_empty());
                    expanded_ambiguous += usize::from(answer.value.len() > 1);
                    if answer.completeness != Completeness::Complete || answer.value.len() != 1 {
                        expanded_ref_rows.push(json!({"relationship":r.relationship.to_string(),"name":r.name.segments,"completeness":format!("{:?}",answer.completeness)}));
                    }
                    expanded_references.query(answer);
                }
                if index % 25 == 0 {
                    println!(
                        "Expanded reference batch {index}: {} checked",
                        expanded_references.count
                    );
                }
            }
            println!(
                "Expanded references: {expanded_unresolved} unresolved, {} incomplete, {expanded_ambiguous} ambiguous",
                expanded_references.incomplete
            );
            let records: Vec<_> = overlay.model().elements().collect();
            for (index, batch) in records.chunks(16).enumerate() {
                let q = KerMlQueries::new(derived_context.fork());
                for record in batch {
                    let subject = record.id();
                    let is = |class| {
                        overlay
                            .model()
                            .registry()
                            .is_subtype(record.metaclass(), class)
                            .unwrap()
                    };
                    checks.validation(q.validate_local_structure(subject));
                    if is(c::NAMESPACE) {
                        checks.validation(q.validate_namespace_distinguishability(subject));
                    }
                    if is(c::FEATURE) {
                        checks.validation(q.validate_formal_target_constraints(subject));
                        checks.query(q.owned_cross_feature(subject));
                        checks.query(q.cross_feature(subject));
                        checks.query(q.feature_target(subject));
                        checks.query(q.feature_types(subject));
                        checks.query(q.featuring_types(subject));
                    }
                    if is(c::FEATURE_VALUE) {
                        checks.query(q.feature_with_value(subject));
                    }
                    if is(c::EXPRESSION) || is(c::FUNCTION) {
                        checks.query(q.structural_result(subject));
                    }
                }
                if index % 10 == 0 {
                    println!(
                        "validation batch {index}: {} queries, {} distinct diagnostics",
                        checks.count,
                        checks.diagnostics.len()
                    );
                }
            }
            (derived_count, None)
        }
        Err(error) => (0, Some(format!("{error:?}"))),
    };
    let diagnostics: Vec<_> = checks
        .diagnostics
        .iter()
        .map(|d| json!({"code":d.code,"subject":d.subject.to_string(),"message":d.message}))
        .collect();
    let distinguishability = checks
        .diagnostics
        .iter()
        .filter(|d| d.code == "validateNamespaceDistinguishibility")
        .count();
    let blockers: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("standards/kerml-1.0-operational-authority-blockers.json"),
    )?)?;
    let report = json!({"format":"agentique-v7-corpus-publication-audit/2","profile":BaselineProfile::OPERATIONAL_V7.id(),
        "library_set":sources.content_set_id(),"canonical_records":ids.len(),"kernel_storage_valid":true,
        "required_references":references.count,"unresolved":unresolved,"incomplete":references.incomplete,
        "ambiguous":ambiguous,"invalid_references":references.invalid,"reference_findings":ref_rows,
        "reference_measurement_phase":"refined source Snapshot before expansion",
        "expanded_references":if source_only { serde_json::Value::Null } else {json!({"count":expanded_references.count,"unresolved":expanded_unresolved,"incomplete":expanded_references.incomplete,"ambiguous":expanded_ambiguous,"invalid":expanded_references.invalid,"findings":expanded_ref_rows})},
        "expanded_occurrences_witnesses":witness_rows,"validation_batch_size":16,
        "planned_derived_records":planned,"derived_overlay_count":derived_count,"overlay_error":overlay_error,
        "producer_complete":!source_only && producer_complete,"producer_diagnostics":producer_diagnostics,
        "query_count":checks.count,"incomplete_queries":checks.incomplete,"invalid_queries":checks.invalid,
        "distinguishability":distinguishability,"validation_findings":diagnostics,"evaluated_rules":checks.rules,
        "authority_blockers":blockers["blockers"].as_array().unwrap().len(),
        "coverage_closed":false,"semantic_publication_accepted":false,"accepted_snapshot_ids":[],
        "validator_scope":if source_only { "SourceSnapshotWithEmptyPartialOverlay" } else { "PartialDerivationOverlay" },
        "deferred_by_phase":["validateElementIsImpliedIncluded"],
        "remaining":"Publication-critical query/producer closure and graph-affecting authority conflicts remain blocking. Validator-only coverage and exhaustive issue applicability are separate from publication (ADR 0022).",
        "execution_implemented":false});
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?
        .write_all(format!("{}\n", serde_json::to_string_pretty(&report)?).as_bytes())?;
    println!(
        "Kernel storage valid; {derived_count} derived records; {distinguishability} distinguishability findings; semantic publication INCOMPLETE"
    );
    Ok(())
}
