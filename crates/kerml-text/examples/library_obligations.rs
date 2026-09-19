//! Gate-zero evidence. This does not accept a candidate or alter semantic rules.
use agq_kerml::{classes as c, properties as p};
use agq_kerml_semantics::{Completeness, QueryResult};
use agq_kerml_syntax::production;
use agq_kerml_text::library::{LibraryDraft, refine_declarations_with_profile};
use agq_kernel::{ElementId, PropertyId, provenance::FactKey};
use agq_standard_libraries::{LibraryLanguage, VerifiedLibrarySet};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fmt::Debug, path::Path};

#[derive(Default)]
struct Evidence {
    values: Vec<String>,
    index: BTreeMap<String, usize>,
}
impl Evidence {
    fn intern(&mut self, value: impl Debug) -> usize {
        let value = format!("{value:?}");
        *self.index.entry(value.clone()).or_insert_with(|| {
            let id = self.values.len();
            self.values.push(value);
            id
        })
    }
    fn result<T: Debug>(&mut self, result: &QueryResult<T>) -> Value {
        json!({
            "candidates":format!("{:?}",result.value),
            "completeness":format!("{:?}",result.completeness),
            "diagnostics":result.diagnostics.iter().map(|d| json!({
                "code":d.code,"subject":d.subject.to_string(),"message":d.message
            })).collect::<Vec<_>>(),
            "positive_dependencies":result.positive_dependencies.iter().map(|v|self.intern(v)).collect::<Vec<_>>(),
            "search_dependencies":result.search_dependencies.iter().map(|v|self.intern(v)).collect::<Vec<_>>(),
            "resolution_explanations":result.explanations.iter()
                .filter(|(claim,_)|claim.query == agq_kerml_semantics::QueryKind::ResolveReference)
                .map(|(claim,proofs)| json!({"subject":claim.subject.to_string(),"target":claim.value.to_string(),
                    "proofs":proofs.iter().map(|proof|json!({"rule":format!("{:?}",proof.rule),
                        "premises":proof.premises.iter().map(|premise|self.intern(premise)).collect::<Vec<_>>()
                    })).collect::<Vec<_>>() })).collect::<Vec<_>>(),
            "rules":result.explanations.values().flatten().map(|e|format!("{:?}",e.rule)).collect::<std::collections::BTreeSet<_>>()
        })
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let sources = VerifiedLibrarySet::load_from_directory(&root)?;
    let profile = if std::env::args().any(|a| a == "--v4") {
        agq_kerml::BaselineProfile::OPERATIONAL_V4
    } else if std::env::args().any(|a| a == "--published") {
        agq_kerml::BaselineProfile::PublishedKerMl10
    } else if std::env::args().any(|a| a == "--v1") {
        agq_kerml::BaselineProfile::OPERATIONAL_V1
    } else if std::env::args().any(|a| a == "--v3") {
        agq_kerml::BaselineProfile::OPERATIONAL_V3
    } else {
        agq_kerml::BaselineProfile::OPERATIONAL
    };
    let draft =
        refine_declarations_with_profile(&sources, profile, |round, endpoints, obligations| {
            println!(
                "refinement {round}: {endpoints} provisional endpoints; {obligations} obligations"
            );
        })?;
    let mut syntax = BTreeMap::new();
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
        for node in parsed.nodes() {
            syntax.insert(node.id(), format!("{:?}", node.kind()));
        }
    }
    let location = |id: ElementId, property: Option<PropertyId>| {
        let model = draft.candidate().model();
        let origin = draft.source_map().get(&FactKey::Element(id));
        let record = model.element(id).unwrap();
        let document = sources
            .documents()
            .find(|d| {
                if let Some(origin) = origin {
                    return d.document() == origin.document;
                }
                matches!(record.origin(), agq_kernel::provenance::Origin::Declared(
                    agq_kernel::provenance::DeclaredOrigin::ReviewedCorrection { source_key, .. }
                ) if source_key == &format!("{}#sha256:{}",d.path(),d.sha256()))
            })
            .unwrap();
        json!({
            "element":id.to_string(),"metaclass":model.registry().class(record.metaclass()).unwrap().name,
            "property":property.map(|p|model.registry().property(p).unwrap().name.clone()),
            "property_id":property.map(|p|p.to_string()),"document":document.path(),"sha256":document.sha256(),
            "source_range":origin.map(|o|[o.range.start(),o.range.end()]),
            "syntax_production":origin.and_then(|o|o.syntax_node).map(|n|syntax[&n].clone()),
            "source":origin.map(|o|&document.source()[o.range.start() as usize..o.range.end() as usize]),
            "provenance":format!("{:?}",record.origin())
        })
    };
    let mut queries = draft.queries(&sources)?;
    let context = queries.context().clone();
    let model = draft.candidate().model();
    let output_path = std::env::args()
        .find_map(|a| a.strip_prefix("--output=").map(str::to_owned))
        .ok_or("--output required")?;
    let phase_path = Path::new(&output_path).with_extension("construction.json");
    serde_json::to_writer_pretty(
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(phase_path)?,
        &json!({"baseline_profile":profile.id(),"rule_version":queries.context().rule_set_version,
            "accepted":false,"mandatory_obligations":draft.candidate().obligations().iter()
                .map(|o|location(o.element,Some(o.property))).collect::<Vec<_>>() }),
    )?;
    let mut evidence = Evidence::default();
    let mut references = vec![];
    let mut state_expression = vec![];
    let mut redefinitions = vec![];
    for (index, reference) in draft.references().iter().enumerate() {
        if index > 0 && index % 128 == 0 {
            queries = draft.queries(&sources)?;
            assert_eq!(queries.context(), &context);
        }
        let result = queries.lookup_relationship_target(
            reference.relationship,
            reference.property,
            &reference.name,
        );
        if reference.property == p::REDEFINITION_REDEFINED_FEATURE {
            let mut row = location(reference.relationship, Some(reference.property));
            row["source_declaration"] = json!(qualified_display(
                &queries,
                model,
                queries
                    .owning_related_element(reference.relationship)
                    .value
                    .unwrap_or(reference.relationship)
            ));
            row["targets"] = json!(result.value.iter().map(|m|json!({"id":m.element.to_string(),"path":qualified_display(&queries,model,m.element)})).collect::<Vec<_>>());
            row["result"] = evidence.result(&result);
            redefinitions.push(row);
        }
        let inspected = reference
            .name
            .segments
            .iter()
            .any(|s| s == "incomingTransitionTrigger");
        if result.value.len() != 1 || result.completeness != Completeness::Complete || inspected {
            let mut row = location(reference.relationship, Some(reference.property));
            row["result"] = evidence.result(&result);
            row["name"] = json!(reference.name.segments);
            row["reference_range"] =
                json!([reference.origin.range.start(), reference.origin.range.end()]);
            row["structure_origin"] = json!("explicit-source");
            row["expression_context"] = json!(reference.executable_expression);
            if inspected {
                state_expression.push(row.clone());
            }
            if result.value.len() != 1 || result.completeness != Completeness::Complete {
                references.push(row);
            }
        }
    }
    let mut semantic = vec![];
    for (index, record) in model.elements().enumerate() {
        if index % 128 == 0 {
            queries = draft.queries(&sources)?;
            assert_eq!(queries.context(), &context);
        }
        let is = |class| {
            model
                .registry()
                .is_subtype(record.metaclass(), class)
                .unwrap()
        };
        let id = record.id();
        let mut check = |kind: &str, result: QueryResult<Vec<ElementId>>| {
            if result.completeness != Completeness::Complete {
                let mut row = location(id, None);
                row["query"] = json!(kind);
                row["result"] = evidence.result(&result);
                row["structure_origin"] = json!("implicit-derived");
                semantic.push(row);
            }
        };
        if is(c::TYPE) {
            check("direct_features", queries.direct_features(id));
            check("direct_specializations", queries.direct_specializations(id));
        }
        if is(c::FEATURE) {
            check("direct_feature_types", queries.direct_feature_types(id));
            check("subsetted_features", queries.subsetted_features(id));
            check("redefined_features", queries.redefined_features(id));
        }
        if is(c::CLASSIFIER) {
            check("all_specializations", queries.all_specializations(id));
            check("effective_features", queries.effective_features(id));
        }
    }
    let obligations: Vec<_> = draft
        .candidate()
        .obligations()
        .iter()
        .map(|o| {
            let mut row = location(o.element, Some(o.property));
            row["required_lower"] = json!(o.required.lower);
            row["actual"] = json!(o.actual);
            row["structure_origin"] = json!(if o.property == p::REDEFINITION_REDEFINED_FEATURE {
                "explicit-source"
            } else {
                "unclassified-mandatory-structural-obligation"
            });
            row
        })
        .collect();
    let strict = strict_publication(&draft);
    let report = json!({
        "format":"agentique-kerml-library-obligations/3", "baseline_profile":profile.id(),
        "library_set":sources.content_set_id(),"rule_version":queries.context().rule_set_version,
        "context":format!("{:?}",queries.context()),
        "evidence_encoding":"Dependency indexes reference evidence_dictionary; debug labels are evidence display, not semantic identity hashing.",
        "evidence_dictionary":evidence.values,"references":references,"semantic_queries":semantic,
        "structural_obligations":obligations,"state_expression_inspection":state_expression,
        "strict_publication_attempt":strict,
        "redefinition_resolutions":redefinitions,
        "publication_accepted":false, "query_cache_batch_size":128
    });
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output_path)?;
    serde_json::to_writer_pretty(file, &report)?;
    println!(
        "Captured {} reference, {} structural, {} query obligations",
        references.len(),
        obligations.len(),
        semantic.len()
    );
    Ok(())
}

fn strict_publication(draft: &LibraryDraft) -> Value {
    use agq_kernel::{Snapshot, provenance::Origin};
    let model = draft.candidate().model();
    let empty = Snapshot::new(std::sync::Arc::new(model.registry().clone()));
    let mut changes = empty.change_set();
    for record in model.elements() {
        let Origin::Declared(origin) = record.origin() else {
            panic!("source construction");
        };
        changes.create(record.id(), record.metaclass(), origin.clone());
        for (property, slot) in record.slots() {
            let Origin::Declared(origin) = slot.origin() else {
                panic!("source slot");
            };
            changes.set(record.id(), property, slot.value().clone(), origin.clone());
        }
    }
    for link in model.association_occurrences() {
        changes.link(
            link.id(),
            link.association(),
            link.ends().clone(),
            link.positions().clone(),
            link.origin().clone(),
        );
    }
    let attempt = empty.apply(&changes);
    assert!(empty.model().is_empty());
    match attempt {
        Ok(snapshot) => {
            json!({"accepted":true,"elements":snapshot.model().elements().count(),"base_unchanged":true})
        }
        Err(error) => json!({"accepted":false,"error":format!("{error:?}"),"base_unchanged":true}),
    }
}

fn qualified_display(
    q: &agq_kerml_semantics::KerMlQueries<'_>,
    model: &agq_kernel::ModelView,
    id: ElementId,
) -> String {
    use agq_kernel::value::Value;
    let mut path = vec![];
    let mut current = Some(id);
    let mut seen = std::collections::BTreeSet::new();
    while let Some(id) = current {
        if !seen.insert(id) {
            break;
        }
        let mut pending = vec![id];
        let mut visited = std::collections::BTreeSet::new();
        let mut names = std::collections::BTreeSet::new();
        while let Some(named) = pending.pop() {
            if !visited.insert(named) {
                continue;
            }
            if let Some(Value::String(name)) = model
                .navigation_slot(named, p::ELEMENT_DECLARED_NAME)
                .and_then(|s| s.value().values().next())
            {
                names.insert(name.clone());
            } else if model
                .registry()
                .is_subtype(model.element(named).unwrap().metaclass(), c::FEATURE)
                .unwrap()
                && let Some(relationship) =
                    q.owned_relationships(named).value.into_iter().find(|r| {
                        model
                            .registry()
                            .is_subtype(model.element(*r).unwrap().metaclass(), c::REDEFINITION)
                            .unwrap()
                    })
                && let Some(Value::Reference(target)) = model
                    .navigation_slot(relationship, p::REDEFINITION_REDEFINED_FEATURE)
                    .and_then(|s| s.value().values().next())
            {
                pending.push(*target);
            }
        }
        if names.len() == 1 {
            path.push(names.into_iter().next().unwrap());
        } else if names.len() > 1 {
            path.push(format!("{names:?}"));
        }
        current = q.owner(id).value;
    }
    path.reverse();
    path.join("::")
}
