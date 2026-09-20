//! Complete canonical fact export for independently checked library corrections.
use agq_kerml::{BaselineProfile, classes as c};
use agq_kernel::{provenance::FactKey, value::Value};
use agq_standard_libraries::VerifiedLibrarySet;
use serde_json::json;
use std::{collections::BTreeMap, path::Path};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sources = VerifiedLibrarySet::load_from_directory(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
    )?;
    let profile = if std::env::args().any(|a| a == "--v5") {
        BaselineProfile::OPERATIONAL_V5
    } else if std::env::args().any(|a| a == "--v4") {
        BaselineProfile::OPERATIONAL_V4
    } else if std::env::args().any(|a| a == "--published") {
        BaselineProfile::PublishedKerMl10
    } else if std::env::args().any(|a| a == "--v1") {
        BaselineProfile::OPERATIONAL_V1
    } else if std::env::args().any(|a| a == "--v3") {
        BaselineProfile::OPERATIONAL_V3
    } else {
        BaselineProfile::OPERATIONAL_V2
    };
    let references_only = std::env::args().any(|a| a == "--references-only");
    let declarations_only = references_only || std::env::args().any(|a| a == "--declarations-only");
    let draft = if declarations_only {
        agq_kerml_text::library::lower_declarations_with_profile(&sources, profile)?
    } else {
        agq_kerml_text::library::refine_declarations_with_profile(
            &sources,
            profile,
            |round, references, obligations| {
                println!("{round}: {references} endpoints; {obligations} obligations")
            },
        )?
    };
    let model = draft.candidate().model();
    let mut queries = if declarations_only {
        None
    } else {
        Some(draft.queries(&sources)?)
    };
    let mut records = BTreeMap::new();
    let mut diagnostics = vec![];
    for (index, record) in model.elements().enumerate() {
        if references_only {
            break;
        }
        if !declarations_only && index % 128 == 0 {
            queries = Some(draft.queries(&sources)?);
        }
        let source = draft
            .source_map()
            .get(&FactKey::Element(record.id()))
            .map(|s| {
                let document = sources
                    .documents()
                    .find(|d| d.document() == s.document)
                    .unwrap();
                json!({"document":document.path(),"sha256":document.sha256(),
                "range":[s.range.start(),s.range.end()],"origin":format!("{s:?}"),
                "text": &document.source()[s.range.start() as usize..s.range.end() as usize]})
            });
        let slots: BTreeMap<_,_> = record.slots().map(|(p,s)| {
            (p.to_string(), json!({"name":model.registry().property(p).unwrap().name,
                "value":format!("{:?}",s.value()),"origin":format!("{:?}",s.origin()),
                "references":s.value().values().filter_map(|v| if let Value::Reference(id)=v {Some(id.to_string())} else {None}).collect::<Vec<_>>() }))
        }).collect();
        records.insert(
            record.id().to_string(),
            json!({"metaclass":model.registry().class(record.metaclass())?.name,
            "metaclass_id":record.metaclass().to_string(),"origin":format!("{:?}",record.origin()),
            "source":source,"slots":slots}),
        );
        if !declarations_only
            && model
                .registry()
                .is_subtype(record.metaclass(), c::NAMESPACE)?
        {
            let validation = queries
                .as_ref()
                .expect("audit queries")
                .validate_namespace_distinguishability(record.id());
            diagnostics.extend(validation.diagnostics.iter().map(
                |d| json!({"code":d.code,"subject":d.subject.to_string(),"message":d.message}),
            ));
        }
    }
    let occurrences: BTreeMap<_, _> = model
        .association_occurrences()
        .map(|r| (r.id().to_string(), format!("{r:?}")))
        .collect();
    let assertions = |references: &[agq_kerml_text::library::PendingLibraryReference]| {
        references.iter().map(|r| json!({
            "relationship":r.relationship.to_string(),"property":r.property.to_string(),
            "expected":r.expected.to_string(),"name":r.name.segments,"absolute":r.name.absolute,
            "membership_target":r.membership_target,"expression_context":r.executable_expression,
            "source_origin":format!("{:?}",r.origin)
        })).collect::<Vec<_>>()
    };
    let output = std::env::args()
        .find_map(|a| a.strip_prefix("--output=").map(str::to_owned))
        .ok_or("--output required")?;
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?;
    serde_json::to_writer_pretty(
        file,
        &json!({"profile":profile.id(),"library_set":sources.content_set_id(),"reference_assertions_only":references_only,
        "records":records,"record_count":model.elements().count(),"occurrences":occurrences,"diagnostics":diagnostics,
        "active_references":assertions(draft.references()),"superseded_references":assertions(draft.superseded_references())}),
    )?;
    if references_only {
        println!(
            "Exported {} active and {} superseded source assertions",
            draft.references().len(),
            draft.superseded_references().len()
        );
    } else {
        println!("Exported {} canonical records", records.len());
    }
    Ok(())
}
