//! Source/identity inspection without resolving or accepting the library corpus.
use agq_kernel::provenance::FactKey;
use agq_standard_libraries::VerifiedLibrarySet;
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = std::env::args()
        .find_map(|a| a.strip_prefix("--input=").map(str::to_owned))
        .ok_or("--input required")?;
    let output = std::env::args()
        .find_map(|a| a.strip_prefix("--output=").map(str::to_owned))
        .ok_or("--output required")?;
    let wanted: BTreeSet<String> = serde_json::from_slice(&std::fs::read(input)?)?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let sources = VerifiedLibrarySet::load_from_directory(&root)?;
    let draft = agq_kerml_text::library::lower_declarations(&sources)?;
    let model = draft.candidate().model();
    let mut rows = vec![];
    let mut classes = BTreeMap::<String, usize>::new();
    for record in model.elements() {
        let class = &model.registry().class(record.metaclass())?.name;
        *classes.entry(class.clone()).or_default() += 1;
        if !wanted.contains(&record.id().to_string()) {
            continue;
        }
        let origin = &draft.source_map()[&FactKey::Element(record.id())];
        let document = sources
            .documents()
            .find(|d| d.document() == origin.document)
            .unwrap();
        rows.push(json!({"id":record.id().to_string(),"class":class,"document":document.path(),
            "range":[origin.range.start(),origin.range.end()],
            "source":&document.source()[origin.range.start() as usize..origin.range.end() as usize]}));
    }
    serde_json::to_writer_pretty(
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output)?,
        &json!({"elements":rows,"classes":classes,"accepted":false}),
    )?;
    println!("Inspected {} source identities", rows.len());
    Ok(())
}
