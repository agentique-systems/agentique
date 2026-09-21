//! Deterministic anchor manifest; this is not whole-library semantic acceptance.
use agq_kerml_text::library::lower_declarations;
use agq_kernel::provenance::FactKey;
use agq_standard_libraries::VerifiedLibrarySet;
use serde_json::json;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().any(|a| a == "--write") {
        return Err("Construction bindings cannot replace the accepted manifest. Use canonical_publication --write-bindings after the publication preflight gates.".into());
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let sources = VerifiedLibrarySet::load_from_directory(&root)?;
    let draft = lower_declarations(&sources)?;
    let queries = draft.queries(&sources)?;
    let bindings = queries
        .context()
        .standard_bindings
        .as_ref()
        .expect("validated bindings");
    let mut entries = vec![];
    for (role, target) in bindings.iter() {
        let (path, metaclass) = role.specification();
        let source = &draft.source_map()[&FactKey::Element(target)];
        let document = sources
            .documents()
            .find(|d| d.document() == source.document)
            .expect("source evidence");
        entries.push(json!({
            "semantic_role": format!("{role:?}"), "library": bindings.bound(role).library.to_string(),
            "qualified_path": path, "expected_metaclass": metaclass.to_string(),
            "expected_metaclass_name": draft.candidate().model().registry().class(metaclass)?.name,
            "element_id": target.to_string(),
            "source": { "document_id": source.document.to_string(), "source_revision_id": source.revision.to_string(),
                "path": document.path(), "sha256": document.sha256(),
                "byte_range": [source.range.start(),source.range.end()], "syntax_node_id": source.syntax_node.map(|n|n.to_string()) }
        }));
    }
    let manifest = json!({
        "format": agq_kerml_semantics::BINDING_VERSION,
        "identity_authority": "Agentique content- and role-qualified IDs; not OMG-assigned semantic IDs",
        "scope": "Validated public canonical anchor declarations; whole-library publication is a separate gate",
        "entries": entries,
    });
    let encoded = format!("{}\n", serde_json::to_string_pretty(&manifest)?);
    let path = root.join("standards/kerml-standard-bindings.json");
    if std::fs::read_to_string(path)?.replace("\r\n", "\n") != encoded {
        return Err("KerML binding manifest is stale".into());
    }
    println!(
        "{} canonical anchor bindings validated",
        bindings.iter().count()
    );
    Ok(())
}
