//! Offline structural-semantic quality gate; candidate construction is not publication.
use agq_kerml::classes as c;
use agq_kerml_semantics::{Completeness, Diagnostic, QueryResult};
use agq_kerml_syntax::production::{self, Production as P};
use agq_kernel::{DocumentId, ElementId, provenance::FactKey};
use agq_standard_libraries::{LibraryLanguage, VerifiedLibrarySet};
use serde_json::{Value, json};
use std::{collections::BTreeMap, io::Write, path::Path};

#[derive(Default)]
struct Counts {
    elements: usize,
    relationships: usize,
    expressions: usize,
    assertions: usize,
    unresolved: usize,
    ambiguous: usize,
    incomplete: usize,
    mismatched: usize,
    queries: usize,
    query_incomplete: usize,
    query_invalid: usize,
    resolution: BTreeMap<(&'static str, ElementId), String>,
    semantic: BTreeMap<(&'static str, ElementId), String>,
    obligations: Vec<Value>,
    validation_checks: BTreeMap<&'static str, usize>,
}
impl Counts {
    fn query<T>(&mut self, result: QueryResult<T>) {
        self.queries += 1;
        self.query_incomplete += usize::from(result.completeness == Completeness::Incomplete);
        self.query_invalid += usize::from(result.completeness == Completeness::Invalid);
        diagnostics(&mut self.semantic, result.diagnostics);
    }
}
fn diagnostics(
    target: &mut BTreeMap<(&'static str, ElementId), String>,
    values: impl IntoIterator<Item = Diagnostic>,
) {
    for d in values {
        target.insert((d.code, d.subject), d.message);
    }
}
fn serialized_diagnostics(values: &BTreeMap<(&'static str, ElementId), String>) -> Vec<Value> {
    values
        .iter()
        .map(|((code, id), message)| {
            json!({
                "code":code, "subject":id.to_string(), "message":message
            })
        })
        .collect()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let sources = VerifiedLibrarySet::load_from_directory(&root)?;
    let profile = if std::env::args().any(|a| a == "--v5") {
        agq_kerml::BaselineProfile::OPERATIONAL_V5
    } else if std::env::args().any(|a| a == "--v4") {
        agq_kerml::BaselineProfile::OPERATIONAL_V4
    } else if std::env::args().any(|a| a == "--published") {
        agq_kerml::BaselineProfile::PublishedKerMl10
    } else if std::env::args().any(|a| a == "--v1") {
        agq_kerml::BaselineProfile::OPERATIONAL_V1
    } else if std::env::args().any(|a| a == "--v3") {
        agq_kerml::BaselineProfile::OPERATIONAL_V3
    } else {
        agq_kerml::BaselineProfile::OPERATIONAL_V2
    };
    let draft = agq_kerml_text::library::refine_declarations_with_profile(
        &sources,
        profile,
        |round, refs, obligations| {
            println!("refinement {round}: {refs} provisional endpoints; {obligations} obligations");
        },
    )?;
    let model = draft.candidate().model();
    let mut queries = draft.queries(&sources)?;
    let context = queries.context().clone();
    let document_of = |element| {
        if let Some(source) = draft.source_map().get(&FactKey::Element(element)) {
            return source.document;
        }
        let agq_kernel::provenance::Origin::Declared(
            agq_kernel::provenance::DeclaredOrigin::ReviewedCorrection { source_key, .. },
        ) = model.element(element).unwrap().origin()
        else {
            panic!("unidentified audit origin")
        };
        sources
            .documents()
            .find(|d| source_key == &format!("{}#sha256:{}", d.path(), d.sha256()))
            .expect("exact correction source identity")
            .document()
    };
    let mut counts = BTreeMap::<DocumentId, Counts>::new();
    for (index, record) in model.elements().enumerate() {
        // Bound memoized proof populations during a complete corpus audit.
        // Rebinding the same immutable input must preserve the exact context.
        if index > 0 && index % 128 == 0 {
            queries = draft.queries(&sources)?;
            assert_eq!(queries.context(), &context);
        }
        let row = counts.entry(document_of(record.id())).or_default();
        let is = |class| {
            model
                .registry()
                .is_subtype(record.metaclass(), class)
                .unwrap()
        };
        row.elements += 1;
        row.relationships += usize::from(is(c::RELATIONSHIP));
        row.expressions += usize::from(is(c::EXPRESSION) || is(c::FUNCTION));
        let local = queries.validate_local_structure(record.id());
        for rule in &local.value {
            *row.validation_checks.entry(rule).or_default() += 1;
        }
        row.query(local);
        if is(c::FEATURE) {
            let targets = queries.validate_formal_target_constraints(record.id());
            for rule in &targets.value {
                *row.validation_checks.entry(rule).or_default() += 1;
            }
            row.query(targets);
        }
        if is(c::NAMESPACE) {
            let names = queries.validate_namespace_distinguishability(record.id());
            for rule in &names.value {
                *row.validation_checks.entry(rule).or_default() += 1;
            }
            row.query(names);
        }
        // These existing query families are actually evaluated. Full KerML
        // constraint validation is a separate, explicitly incomplete scope.
        if is(c::TYPE) {
            row.query(queries.direct_features(record.id()));
            row.query(queries.direct_specializations(record.id()));
        }
        if is(c::FEATURE) {
            row.query(queries.direct_feature_types(record.id()));
            row.query(queries.subsetted_features(record.id()));
            row.query(queries.redefined_features(record.id()));
        }
        if is(c::CLASSIFIER) {
            row.query(queries.all_specializations(record.id()));
            row.query(queries.effective_features(record.id()));
        }
    }
    println!("Existing structural query families evaluated over candidate declarations");
    for (index, reference) in draft.references().iter().enumerate() {
        if index % 128 == 0 {
            queries = draft.queries(&sources)?;
            assert_eq!(queries.context(), &context);
        }
        let row = counts.get_mut(&reference.origin.document).unwrap();
        let result = queries.lookup_relationship_target(
            reference.relationship,
            reference.property,
            &reference.name,
        );
        row.assertions += 1;
        row.unresolved += usize::from(result.value.is_empty());
        row.ambiguous += usize::from(result.value.len() > 1);
        row.incomplete += usize::from(result.completeness != Completeness::Complete);
        if let [member] = result.value.as_slice() {
            let target = if reference.membership_target {
                member.membership
            } else {
                member.element
            };
            let correct_kind = model.registry().is_subtype(
                model.element(target).unwrap().metaclass(),
                reference.expected,
            )?;
            let correct_value = model
                .navigation_slot(reference.relationship, reference.property)
                .is_some_and(|slot| {
                    slot.value().values().any(
                        |v| matches!(v, agq_kernel::value::Value::Reference(id) if *id == target),
                    )
                });
            row.mismatched += usize::from(!correct_kind || !correct_value);
        }
        if result.value.len() != 1 {
            row.resolution.insert(
                (
                    if result.value.is_empty() {
                        "KLS_UNRESOLVED"
                    } else {
                        "KLS_AMBIGUOUS"
                    },
                    reference.relationship,
                ),
                format!(
                    "{} at bytes {}..{}; expression context: {}",
                    reference.name.segments.join("::"),
                    reference.origin.range.start(),
                    reference.origin.range.end(),
                    reference.executable_expression
                ),
            );
        }
        diagnostics(&mut row.resolution, result.diagnostics);
    }
    for obligation in draft.candidate().obligations() {
        let origin = draft
            .source_map()
            .get(&FactKey::Element(obligation.element));
        counts.get_mut(&document_of(obligation.element)).unwrap().obligations.push(json!({
            "code":"KLS_STRUCTURAL_OBLIGATION", "element":obligation.element.to_string(),
            "property":model.registry().property(obligation.property)?.name,
            "property_id":obligation.property.to_string(), "required_lower":obligation.required.lower,
            "actual":obligation.actual, "range":origin.map(|o| [o.range.start(),o.range.end()]),
            "provenance":format!("{:?}",model.element(obligation.element).unwrap().origin())
        }));
    }
    let mut documents = vec![];
    for source in sources
        .documents()
        .filter(|d| d.language() == LibraryLanguage::KerMl)
    {
        let parsed = production::parse(
            source.document(),
            source.revision(),
            source.source(),
            Default::default(),
        )?;
        let round_trip: String = parsed
            .tokens()
            .iter()
            .map(|t| parsed.token_text(t))
            .collect();
        let preserved = round_trip == source.source();
        let row = &counts[&source.document()];
        let syntax: Vec<_> = parsed.diagnostics().iter().map(|d| json!({"code":d.code,"range":[d.range.start(),d.range.end()],"message":d.message})).collect();
        let mut anomalies: Vec<_> = parsed.discrepancies().iter().map(|d| json!({"code":d.code,"range":[d.range.start(),d.range.end()],"authority":d.authority})).collect();
        anomalies.extend(parsed.nodes().filter(|n| n.kind() == P::TypeResultMember).map(|n| json!({
            "code":"KLS_CAST_RESULT_DEFAULT", "range":[n.range().start(),n.range().end()],
            "authority":"KerML 1.0 pp.95-96; validateExpressionResultParameterMembership; ADR 0013",
            "interpretation":"Typed result satisfies the appended empty-result default; both syntax productions retained"
        })));
        let resolution_complete =
            row.unresolved == 0 && row.ambiguous == 0 && row.incomplete == 0 && row.mismatched == 0;
        println!(
            "{}: {} candidate elements, {} unresolved, {} incomplete reference answers",
            source.path(),
            row.elements,
            row.unresolved,
            row.incomplete
        );
        documents.push(json!({
            "file":source.path(), "sha256":source.sha256(), "library_id":source.library().to_string(),
            "document_id":source.document().to_string(), "source_revision_id":source.revision().to_string(),
            "syntax_status":if parsed.is_complete() { "parsed" } else { "recovered" },
            "tokens":parsed.tokens().len(), "syntax_nodes":parsed.nodes().count(),
            "recovery_count":parsed.recovery().len(), "unsupported_syntax":syntax.len(),
            "source_coverage_percent":if preserved { Some(100) } else { None }, "exact_source_preservation":preserved,
            "canonical_element_count":row.elements, "canonical_relationship_count":row.relationships,
            "count_scope":"unpublished ordinary kernel construction; not an accepted Snapshot",
            "resolution_status":if resolution_complete { "complete" } else { "incomplete" },
            "reference_count":row.assertions,"unresolved_count":row.unresolved,"ambiguous_count":row.ambiguous,
            "incomplete_reference_count":row.incomplete,"mismatched_endpoint_count":row.mismatched,
            "KerML_semantic_status":"incomplete", "full_KerML_constraint_validation":{"status":"incomplete","evaluated_constraints":row.validation_checks,"remaining_scope":"Additional structural, derived and implied relationship constraints remain mandatory before acceptance"},
            "query_count":row.queries, "incomplete_query_count":row.query_incomplete, "invalid_query_count":row.query_invalid,
            "unevaluated_expression_count":row.expressions,
            "diagnostics_by_category":{
                "syntax":syntax, "published_source_anomaly":anomalies,
                "resolution":serialized_diagnostics(&row.resolution),
                "KerML_semantic":{"query_diagnostics":serialized_diagnostics(&row.semantic),"structural_obligations":row.obligations,
                    "validation_scope":"Direct feature/specialization/typing/subsetting/redefinition queries on all applicable elements; transitive specialization/effective features on classifiers. Remaining language constraints not evaluated."},
                "unevaluated_expression_function_semantics":{"count":row.expressions,"code":"KLS_EVALUATION_UNSUPPORTED","scope":"All expression/function execution; structural references are still checked separately"}
            }
        }));
    }
    documents.sort_by_key(|d| d["file"].as_str().unwrap().to_owned());
    assert_eq!(documents.len(), 36);
    // Full validation and strict publication remain mandatory. Successful
    // construction or a set of provisional endpoint candidates cannot pass.
    let published = false;
    let passed = published
        && documents.iter().all(|d| {
            d["syntax_status"] == "parsed"
                && d["exact_source_preservation"] == true
                && d["resolution_status"] == "complete"
                && d["KerML_semantic_status"] == "complete"
        });
    let report = json!({
        "format":"agentique-kerml-library-semantic-quality/1", "library_set":sources.content_set_id(),
        "baseline_profile":draft.baseline_profile().id(),
        "authority_scope":"Agentique operational errata profile; exact published descriptors remain separately accessible",
        "scope":"Complete pinned three-library KerML corpus. Structural semantic publication required; execution excluded. Unevaluated validation scopes are explicit.",
        "query_cache_batch_size":128,
        "semantic_quality_gate_passed":passed,"published_snapshot":published,
        "structural_obligation_count":draft.candidate().obligations().len(),
        "binding_count":queries.context().standard_bindings.as_ref().unwrap().iter().count(),
        "binding_version":queries.context().binding_version,"rule_version":queries.context().rule_set_version,
        "library_graph_digest":queries.context().library_graph_digest,
        "documents":documents
    });
    if let Some(path) =
        std::env::args().find_map(|a| a.strip_prefix("--output=").map(str::to_owned))
    {
        let bytes = format!("{}\n", serde_json::to_string_pretty(&report)?);
        if std::env::args().any(|a| a == "--check") {
            if std::fs::read(path)? != bytes.as_bytes() {
                return Err("stale quality report".into());
            }
        } else {
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)?
                .write_all(bytes.as_bytes())?;
        }
    }
    println!(
        "KerML semantic quality: {}",
        if passed { "PASS" } else { "INCOMPLETE" }
    );
    if !passed {
        std::process::exit(1);
    }
    Ok(())
}
