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
            "rules":result.explanations.values().flatten().map(|e|format!("{:?}",e.rule)).collect::<std::collections::BTreeSet<_>>()
        })
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let sources = VerifiedLibrarySet::load_from_directory(&root)?;
    let profile = if std::env::args().any(|a| a == "--published") {
        agq_kerml::BaselineProfile::PublishedKerMl10
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
        let origin = &draft.source_map()[&FactKey::Element(id)];
        let document = sources
            .documents()
            .find(|d| d.document() == origin.document)
            .unwrap();
        let record = model.element(id).unwrap();
        json!({
            "element":id.to_string(),"metaclass":model.registry().class(record.metaclass()).unwrap().name,
            "property":property.map(|p|model.registry().property(p).unwrap().name.clone()),
            "property_id":property.map(|p|p.to_string()),"document":document.path(),"sha256":document.sha256(),
            "source_range":[origin.range.start(),origin.range.end()],
            "syntax_production":origin.syntax_node.map(|n|syntax[&n].clone()),
            "source":&document.source()[origin.range.start() as usize..origin.range.end() as usize]
        })
    };
    let queries = draft.queries(&sources)?;
    let model = draft.candidate().model();
    let mut evidence = Evidence::default();
    let mut references = vec![];
    let mut state_expression = vec![];
    for reference in draft.references() {
        let result = queries.lookup_relationship_target(
            reference.relationship,
            reference.property,
            &reference.name,
        );
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
    for record in model.elements() {
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
                "authority-conflict: no normative derivation identified"
            });
            row
        })
        .collect();
    let strict = strict_publication(&draft);
    let report = json!({
        "format":"agentique-kerml-library-obligations/3", "baseline_profile":profile.id(),
        "library_set":sources.content_set_id(),"rule_version":queries.context().rule_set_version,
        "evidence_encoding":"Dependency indexes reference evidence_dictionary; debug labels are evidence display, not semantic identity hashing.",
        "evidence_dictionary":evidence.values,"references":references,"semantic_queries":semantic,
        "structural_obligations":obligations,"state_expression_inspection":state_expression,
        "strict_publication_attempt":strict,
        "publication_accepted":false
    });
    let path = std::env::args()
        .find_map(|a| a.strip_prefix("--output=").map(str::to_owned))
        .ok_or("--output required")?;
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
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
    let error = empty.apply(&changes).expect_err("Baseline cannot publish");
    assert!(empty.model().is_empty());
    json!({"accepted":false,"error":format!("{error:?}"),"base_unchanged":true})
}
