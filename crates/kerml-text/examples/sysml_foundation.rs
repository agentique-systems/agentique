//! Real accepted-dependency SysML vertical and exact-source candidate inventory.
use agq_kerml::{BaselineProfile, properties as p};
use agq_kerml_semantics::{Completeness, QualifiedName, Resolution, StandardRole};
use agq_kerml_text::{
    ProjectChange, ProjectRevision, SourceLanguage, SourceProject,
    library::CanonicalKermlStandardLibraries, sysml::prepare_systems_library,
};
use agq_kernel::{ElementId, Snapshot, value::Value};
use agq_standard_libraries::VerifiedLibrarySet;
use agq_sysml::classes as s;
use agq_sysml_semantics::{
    StandardSysmlBindings, StandardSysmlRole, SysmlDependencyContract, SysmlQueries,
    SysmlSemanticContext, SystemsLibraryIdentity,
};
use serde_json::json;
use std::{collections::BTreeSet, io::Write, path::Path, sync::Arc, time::Instant};

#[path = "support/sysml_programmatic.rs"]
mod programmatic;

fn local_named(snapshot: &Snapshot, name: &str) -> ElementId {
    let ids: Vec<_> = snapshot
        .model()
        .elements()
        .filter(|record| {
            !snapshot.is_dependency_element(record.id())
                && snapshot
                    .model()
                    .navigation_slot(record.id(), p::ELEMENT_DECLARED_NAME)
                    .is_some_and(|slot| {
                        slot.value()
                            .values()
                            .any(|value| value == &Value::String(name.into()))
                    })
        })
        .map(|record| record.id())
        .collect();
    assert_eq!(ids.len(), 1, "local declaration {name}");
    ids[0]
}

fn attach<'m>(
    snapshot: &'m Snapshot,
    root: ElementId,
    publication: &CanonicalKermlStandardLibraries,
    expected: &SysmlDependencyContract,
    bindings: &StandardSysmlBindings,
) -> Result<SysmlQueries<'m>, Box<dyn std::error::Error>> {
    Ok(SysmlQueries::new(SysmlSemanticContext::for_project(
        snapshot,
        publication.complete_overlay(),
        root,
        BTreeSet::new(),
        BTreeSet::new(),
        expected,
        bindings.clone(),
    )?))
}

fn assert_vertical(queries: &SysmlQueries<'_>, ids: [ElementId; 4]) {
    let [engine_definition, vehicle, sports_car, engine] = ids;
    let count = queries.model().elements().count();
    for definition in [engine_definition, vehicle, sports_car] {
        assert_eq!(
            queries.model().element(definition).unwrap().metaclass(),
            s::PART_DEFINITION
        );
    }
    assert_eq!(
        queries.model().element(engine).unwrap().metaclass(),
        s::PART_USAGE
    );
    let types = queries.direct_usage_types(engine);
    assert_eq!(
        types.completeness(),
        Completeness::Complete,
        "{:?}",
        types.diagnostics
    );
    assert_eq!(types.value(), &[engine_definition]);
    let current_types = queries.current_usage_types(engine);
    assert_eq!(current_types.completeness(), Completeness::Complete);
    assert_eq!(current_types.value(), &[engine_definition]);
    let part_types = queries.current_part_definitions(engine);
    assert_eq!(
        part_types.completeness(),
        Completeness::Complete,
        "{:?}",
        part_types.diagnostics
    );
    assert_eq!(part_types.value(), &[engine_definition]);
    let supers = queries.direct_specializations(sports_car);
    assert_eq!(
        supers.completeness(),
        Completeness::Complete,
        "{:?}",
        supers.diagnostics
    );
    assert_eq!(supers.value(), &[vehicle]);
    let inherited = queries.current_effective_usages(sports_car);
    assert_eq!(
        inherited.completeness(),
        Completeness::Complete,
        "{:?}",
        inherited.diagnostics
    );
    assert_eq!(
        inherited.value(),
        &[engine],
        "inherited usage retains its original identity"
    );
    let owned = queries.owned_usages(sports_car);
    assert_eq!(owned.completeness(), Completeness::Complete);
    assert!(owned.value().is_empty());
    let name = queries.current_qualified_name(engine);
    assert_eq!(name.completeness(), Completeness::Complete);
    assert_eq!(
        name.value().as_ref().unwrap().segments,
        vec![
            BTreeSet::from(["Vehicle".into()]),
            BTreeSet::from(["engine".into()])
        ]
    );
    assert_eq!(
        queries.model().elements().count(),
        count,
        "queries never copy inherited usages"
    );
    let effective = queries.effective_usages(sports_car);
    assert_eq!(effective.completeness(), Completeness::Incomplete);
    assert!(
        !effective.pending.is_empty(),
        "unfinished SysML producers stay explicit"
    );
}

fn require_complete(revision: &ProjectRevision) {
    assert!(
        revision.is_complete_slice(),
        "{:?}",
        revision.semantic_diagnostics()
    );
    assert!(revision.references().iter().all(|reference| {
        reference.resolution.completeness == Completeness::Complete
            && matches!(reference.resolution.value, Resolution::Resolved(_))
    }));
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let started = Instant::now();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let cache = std::env::args()
        .find_map(|arg| arg.strip_prefix("--cache=").map(|p| root.join(p)))
        .ok_or("--cache=<accepted publication ZIP> is required")?;
    let output = std::env::args()
        .find_map(|arg| arg.strip_prefix("--output=").map(|p| root.join(p)))
        .ok_or("--output=<fresh report JSON> is required")?;
    std::fs::create_dir_all(output.parent().ok_or("output parent")?)?;
    let mut report = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&output)?;
    let sources = VerifiedLibrarySet::load_from_directory(&root)?;
    println!("SysML foundation: restoring accepted KerML dependency");
    let publication = Arc::new(CanonicalKermlStandardLibraries::restore_cache(
        std::fs::File::open(&cache)?,
        &sources,
    )?);
    println!(
        "SysML foundation: accepted dependency restored in {:.3}s",
        started.elapsed().as_secs_f64()
    );
    let mut content = [0; 32];
    let content_hex = sources
        .content_set_id()
        .strip_prefix("sha256:")
        .ok_or("source content set must be a SHA-256 identity")?;
    if content_hex.len() != 64 {
        return Err("source content set digest must contain 64 hexadecimal digits".into());
    }
    for (index, byte) in content.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&content_hex[index * 2..index * 2 + 2], 16)?;
    }
    let systems_identity = SystemsLibraryIdentity::pinned(content);
    let bindings = StandardSysmlBindings::unbound(systems_identity.clone());
    let contract = SysmlDependencyContract::checked_in(&bindings)?;
    let mut project = SourceProject::with_sysml_standard_libraries(publication.clone())?;
    let revision = project.apply(project.current().revision(), [ProjectChange::Add {
        path: "vehicle.sysml".into(), language: SourceLanguage::SysMl,
        source: "part def Engine; part def Vehicle { part engine : Engine; } part def SportsCar :> Vehicle;".into(),
    }])?;
    require_complete(&revision);
    let ids = ["Engine", "Vehicle", "SportsCar", "engine"]
        .map(|name| local_named(revision.snapshot(), name));
    let queries = attach(
        revision.snapshot(),
        revision.root(),
        &publication,
        &contract,
        &bindings,
    )?;
    assert_vertical(&queries, ids);
    assert_eq!(
        queries.kerml().direct_feature_types(ids[3]).value,
        vec![ids[0]]
    );
    let all_features = queries.kerml().effective_features(ids[2]);
    assert_eq!(all_features.completeness, Completeness::Complete);
    assert!(
        all_features.value.contains(&ids[3]),
        "KerML inheritance includes the original authored usage alongside library Features"
    );
    assert_eq!(
        queries.context().dependencies.kerml_publication_digest,
        publication.semantic_digest()
    );
    println!("SysML foundation: authored vertical complete");

    let base = Snapshot::with_immutable_dependency_in_registry(
        revision.snapshot().immutable_dependency().unwrap().clone(),
        Arc::new(agq_sysml::registry_for_profile(
            BaselineProfile::OPERATIONAL_V9,
        )?),
    )?;
    let (direct, [direct_root, engine, vehicle, sports_car, usage]) =
        programmatic::construct(base)?;
    let direct_queries = attach(&direct, direct_root, &publication, &contract, &bindings)?;
    assert_vertical(&direct_queries, [engine, vehicle, sports_car, usage]);
    assert_ne!(
        engine, ids[0],
        "equivalence does not require authored syntax IDs"
    );
    assert!(Arc::ptr_eq(
        direct.immutable_dependency().unwrap(),
        revision.snapshot().immutable_dependency().unwrap()
    ));
    println!("SysML foundation: independent kernel equivalent complete");

    for field in ["digest", "profile", "libraries", "descriptors"] {
        let mut wrong = contract.clone();
        match field {
            "digest" => wrong.kerml_publication_digest[0] ^= 1,
            "profile" => wrong.kerml_profile = BaselineProfile::OPERATIONAL_V8.id().into(),
            "libraries" => wrong.kerml_libraries.pins.clear(),
            "descriptors" => wrong.combined_descriptor_digest[0] ^= 1,
            _ => unreachable!(),
        }
        assert!(
            SysmlSemanticContext::for_project(
                revision.snapshot(),
                publication.complete_overlay(),
                revision.root(),
                BTreeSet::new(),
                BTreeSet::new(),
                &wrong,
                bindings.clone(),
            )
            .is_err(),
            "reject changed {field}"
        );
    }
    drop(direct_queries);
    drop(direct);
    drop(queries);

    println!("SysML foundation: mixed authored resolution and edit isolation");
    let mixed = project.apply(revision.revision(), [
        ProjectChange::Add { path:"support.kerml".into(), language:SourceLanguage::KerMl,
            source:"package Support { public class RootClass; public alias Universe for Base::Anything; }".into() },
        ProjectChange::Add { path:"mixed.sysml".into(), language:SourceLanguage::SysMl,
            source:"package Mixed { private import Support::*; part def Derived :> Support::RootClass; }".into() },
    ])?;
    require_complete(&mixed);
    assert_eq!(
        ids,
        ["Engine", "Vehicle", "SportsCar", "engine"]
            .map(|name| local_named(mixed.snapshot(), name))
    );
    let q = mixed.queries();
    let universe = q.lookup_path(
        mixed.root(),
        &QualifiedName {
            absolute: false,
            segments: ["Support", "Universe"].map(String::from).to_vec(),
        },
    );
    assert_eq!(universe.completeness, Completeness::Complete);
    assert_eq!(
        universe.value[0].element,
        publication.bindings().get(StandardRole::Anything)
    );
    drop(q);
    let vehicle_source = mixed.document_at("vehicle.sysml").unwrap();
    let end = vehicle_source.source().len() as u64;
    let edited = project.apply(
        mixed.revision(),
        [ProjectChange::Edit {
            document: vehicle_source.id(),
            edit: agq_kerml_text::syntax::TextEdit {
                range: agq_kernel::provenance::ByteRange::new(end, end).unwrap(),
                replacement: " part def Trailer;".into(),
            },
        }],
    )?;
    require_complete(&edited);
    assert_eq!(
        ids,
        ["Engine", "Vehicle", "SportsCar", "engine"]
            .map(|name| local_named(edited.snapshot(), name))
    );
    let before_rejection = project.current().revision();
    assert!(
        project
            .apply(
                before_rejection,
                [ProjectChange::Add {
                    path: "unsupported.sysml".into(),
                    language: SourceLanguage::SysMl,
                    source: "action def Work;".into(),
                }]
            )
            .is_err()
    );
    assert_eq!(project.current().revision(), before_rejection);
    assert_eq!(revision.documents().count(), 1);
    let authored_elements = mixed
        .snapshot()
        .model()
        .elements()
        .filter(|record| !mixed.snapshot().is_dependency_element(record.id()))
        .count();
    drop(edited);
    drop(mixed);
    drop(revision);
    drop(project);

    println!("SysML foundation: strict Systems Library candidate construction");
    let candidate = prepare_systems_library(&sources, publication.clone())?;
    assert_eq!(candidate.documents().len(), 21);
    assert!(
        candidate
            .documents()
            .iter()
            .all(|document| document.byte_exact)
    );
    let candidate_q = candidate.queries()?;
    let anchor_validation = StandardSysmlBindings::validate(
        candidate.draft().candidate().model(),
        &candidate_q,
        systems_identity,
        candidate.draft().roots(),
        StandardSysmlRole::ALL,
    );
    let parsed = candidate
        .documents()
        .iter()
        .filter(|document| document.parsed)
        .count();
    let lowered = candidate
        .documents()
        .iter()
        .filter(|document| document.construction_gap.is_none())
        .count();
    let status = json!({
        "format":"agq-sysml-foundation-check/1",
        "accepted_kerml_digest":publication.semantic_digest(),
        "accepted_profile":publication.profile().id(),
        "restored_from_checked_in_receipt":publication.complete_overlay().restored_from_receipt(),
        "producer_closure_rerun":false,
        "authored_vertical":true,
        "programmatic_equivalence":true,
        "inherited_usage_copied":false,
        "dependency_contract_rejections":4,
        "mixed_kerml_sysml_resolution":true,
        "authored_edit_isolation":true,
        "authored_local_elements":authored_elements,
        "systems_documents_parsed":parsed,
        "systems_documents_constructed":lowered,
        "systems_documents_byte_exact":21,
        "systems_construction_complete":candidate.construction_complete(),
        "systems_kernel_obligations":candidate.draft().candidate().obligations().len(),
        "systems_reference_assertions":candidate.draft().references().len(),
        "systems_anchor_validation":anchor_validation.as_ref().map(|b| b.targets().len()).map_err(ToString::to_string),
        "systems_publication_accepted":false,
        "sysml_producer_closure_complete":false,
        "status":"SYSML SYSTEMS LIBRARY FOUNDATION INCOMPLETE",
        "elapsed_seconds":started.elapsed().as_secs_f64(),
        "documents":candidate.documents().iter().map(|document| json!({
            "path":document.path, "document":document.document, "source_sha256":document.source_sha256,
            "parsed":document.parsed, "byte_exact":document.byte_exact, "construction_gap":document.construction_gap,
        })).collect::<Vec<_>>(),
    });
    report.write_all(&serde_json::to_vec_pretty(&status)?)?;
    report.sync_all()?;
    println!(
        "SYSML SYSTEMS LIBRARY FOUNDATION INCOMPLETE: {parsed}/21 strict parses, {lowered}/21 constructed; authored vertical and equivalence passed"
    );
    Ok(())
}
