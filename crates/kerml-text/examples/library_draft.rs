use agq_kerml_text::library::lower_declarations;
use agq_standard_libraries::VerifiedLibrarySet;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let sources = VerifiedLibrarySet::load_from_directory(&root)?;
    let draft = if std::env::args().any(|a| a == "--resolve") {
        agq_kerml_text::library::refine_declarations(&sources, |round, resolved, obligations| {
            println!(
                "round {round}: {resolved} provisional targets; {obligations} structural obligations"
            );
        })?
    } else {
        lower_declarations(&sources)?
    };
    println!(
        "{} canonical candidate elements, {} roots, {} reference assertions, {} unmet structural obligations",
        draft.candidate().model().len(),
        draft.roots().len(),
        draft.references().len(),
        draft.candidate().obligations().len()
    );
    let mut targets = std::collections::BTreeMap::new();
    for reference in draft.references() {
        let key = (reference.relationship, reference.property);
        if let Some(previous) = targets.insert(key, reference) {
            println!("DUPLICATE {previous:?} {reference:?}");
        }
    }
    let mut obligations = std::collections::BTreeMap::new();
    for obligation in draft.candidate().obligations() {
        let property = draft
            .candidate()
            .model()
            .registry()
            .property(obligation.property)?;
        *obligations.entry(property.name.clone()).or_insert(0) += 1;
        if !targets.contains_key(&(obligation.element, obligation.property)) {
            let origin =
                &draft.source_map()[&agq_kernel::provenance::FactKey::Element(obligation.element)];
            println!("Additional obligation {obligation:?} at {origin:?}");
        }
    }
    println!("Obligations by property: {obligations:?}");
    if let Some(path) =
        std::env::args().find_map(|a| a.strip_prefix("--report=").map(str::to_string))
    {
        let queries = draft.queries(&sources)?;
        let model = draft.candidate().model();
        let mut reports = vec![];
        for obligation in draft.candidate().obligations() {
            let record = model.element(obligation.element).unwrap();
            let class = model.registry().class(record.metaclass())?;
            let property = model.registry().property(obligation.property)?;
            let reference = targets.get(&(obligation.element, obligation.property));
            let origin = reference.map(|r| &r.origin).unwrap_or(
                &draft.source_map()[&agq_kernel::provenance::FactKey::Element(obligation.element)],
            );
            let document = sources
                .documents()
                .find(|d| d.document() == origin.document)
                .unwrap();
            let result = reference
                .map(|r| queries.lookup_relationship_target(r.relationship, r.property, &r.name));
            reports.push(serde_json::json!({
                "class": class.name, "property": property.name, "path": document.path(),
                "range": [origin.range.start(), origin.range.end()],
                "source": &document.source()[origin.range.start() as usize..origin.range.end() as usize],
                "expression": reference.map(|r| r.executable_expression),
                "candidates": result.as_ref().map(|r| r.value.iter().map(|m| format!("{:?}", m)).collect::<Vec<_>>()),
                "completeness": result.as_ref().map(|r| format!("{:?}", r.completeness)),
                "diagnostics": result.as_ref().map(|r| r.diagnostics.iter().map(|d| d.code).collect::<std::collections::BTreeSet<_>>()),
            }));
        }
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        serde_json::to_writer_pretty(file, &reports)?;
    }
    Ok(())
}
