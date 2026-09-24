//! Read-only diagnosis of an obsolete frontier; never publication authority.
use agq_kerml::{classes as kc, properties as kp};
use agq_kerml_semantics::PublicationFrontierSession;
use agq_kerml_text::library::CanonicalKermlStandardLibraries;
use agq_kernel::{ElementId, value::Value};
use agq_standard_libraries::VerifiedLibrarySet;
use agq_sysml_semantics::{
    StandardSysmlBindings, SysmlBaselineProfile, SysmlDependencyContract, SysmlQueries,
    SysmlSemanticContext, SystemsLibraryIdentity,
};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, path::PathBuf, sync::Arc};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let root = PathBuf::from(args.first().ok_or("repository root required")?);
    let journal = PathBuf::from(args.get(1).ok_or("journal required")?);
    let cache = PathBuf::from(args.get(2).ok_or("accepted KerML cache required")?);
    let sources = VerifiedLibrarySet::load_from_directory(&root)?;
    eprintln!("Restoring accepted KerML dependency");
    let accepted = Arc::new(CanonicalKermlStandardLibraries::restore_cache(
        std::fs::File::open(cache)?,
        &sources,
    )?);
    let bytes = std::fs::read(&journal)?;
    let metadata: serde_json::Value = serde_json::from_slice(&bytes)?;
    let source_identity: [u8; 32] = serde_json::from_value(metadata["source_identity"].clone())?;
    // The caller chooses a retained report for diagnosis. This intentionally
    // authenticates stored bytes only and issues no current producer certificate.
    let session = PublicationFrontierSession::resume(
        &journal,
        Sha256::digest(&bytes).into(),
        source_identity,
        0,
    )?;
    eprintln!("Restoring unaccepted graph for diagnosis only");
    let frontier = session.restore_converged_frontier(
        Arc::new(agq_sysml::registry_for_profile(accepted.profile())?),
        Some(Arc::new(accepted.complete_overlay().overlay().clone())),
    )?;
    let bindings = StandardSysmlBindings::unbound(SystemsLibraryIdentity::pinned(
        SystemsLibraryIdentity::SOURCE_CONTENT_SET,
    ));
    let contract = SysmlDependencyContract::checked_in_for_profile(
        &bindings,
        SysmlBaselineProfile::OperationalV2,
    )?;
    let context = SysmlSemanticContext::for_producer_overlay(
        frontier.overlay(),
        accepted.complete_overlay(),
        &[],
        &contract,
        bindings,
    )?;
    let q = SysmlQueries::new(context);
    let k = q.kerml();
    let model = frontier.overlay().model();
    let name = |id| {
        model
            .navigation_slot(id, kp::ELEMENT_DECLARED_NAME)
            .and_then(|s| {
                s.value().values().find_map(|v| match v {
                    Value::String(s) => Some(s.clone()),
                    _ => None,
                })
            })
            .unwrap_or_default()
    };
    let subjects: Vec<_> = model
        .elements()
        .filter(|e| {
            model
                .registry()
                .is_subtype(e.metaclass(), agq_sysml::classes::FLOW_USAGE)
                .unwrap_or(false)
                && ["messages", "flows", "successionFlows"].contains(&name(e.id()).as_str())
        })
        .map(|e| e.id())
        .collect();
    for subject in subjects {
        let actual = q.effective_parameters(subject);
        println!(
            "{}",
            serde_json::json!({"subject":subject,"name":name(subject),
            "effective_completeness":format!("{:?}",actual.completeness()),
            "effective_value":actual.value(),
            "effective_diagnostics":format!("{:?}",actual.diagnostics),
            "kernel_diagnostics":format!("{:?}",actual.kerml.diagnostics),"publication_authority":false})
        );
        let mut pending = vec![subject];
        let mut seen = BTreeSet::new();
        while let Some(current) = pending.pop() {
            if !seen.insert(current) {
                continue;
            }
            let owned = k.owned_parameter_features(current);
            let mut generals = k.owned_specialization_targets(current).value;
            let relationships: Vec<ElementId> = model
                .navigation_slot(current, kp::ELEMENT_OWNED_RELATIONSHIP)
                .into_iter()
                .flat_map(|s| s.value().values())
                .filter_map(|v| match v {
                    Value::Reference(id) => Some(*id),
                    _ => None,
                })
                .collect();
            for relationship in relationships {
                let class = model.element(relationship).unwrap().metaclass();
                for (kind, property) in [
                    (kc::CONJUGATION, kp::CONJUGATION_ORIGINAL_TYPE),
                    (kc::FEATURE_CHAINING, kp::FEATURE_CHAINING_CHAINING_FEATURE),
                ] {
                    if model.registry().is_subtype(class, kind)? {
                        if kind == kc::CONJUGATION {
                            generals.clear();
                        }
                        generals.extend(
                            model
                                .navigation_slot(relationship, property)
                                .into_iter()
                                .flat_map(|s| s.value().values())
                                .filter_map(|v| match v {
                                    Value::Reference(id) => Some(*id),
                                    _ => None,
                                }),
                        );
                    }
                }
            }
            generals.retain(|g| *g != current);
            println!(
                "{}",
                serde_json::json!({"root":subject,"node":current,
                "name":name(current),"owned":owned.value,"owned_completeness":format!("{:?}",owned.completeness),
                "generals":generals,"owned_diagnostics":format!("{:?}",owned.diagnostics)})
            );
            pending.extend(generals);
        }
    }
    let classification =
        root.join("verification/fixtures/final-audit-semantic-closure/prior-classification.json");
    if classification.exists() {
        let report: serde_json::Value = serde_json::from_slice(&std::fs::read(classification)?)?;
        let subjects: BTreeSet<ElementId> = report["diagnostics"]
            .as_array()
            .ok_or("diagnostics missing")?
            .iter()
            .filter(|d| d["category"] == "ResultCycle")
            .map(|d| serde_json::from_value(d["subject"].clone()))
            .collect::<Result<_, _>>()?;
        for subject in subjects {
            let actual = k.result_parameters(subject);
            let mut owners = Vec::new();
            let mut current = subject;
            let mut seen = BTreeSet::new();
            while seen.insert(current) {
                owners.push(serde_json::json!({"id":current,"name":name(current)}));
                let Some(owner) = model
                    .navigation_slot(current, kp::ELEMENT_OWNING_RELATIONSHIP)
                    .and_then(|s| {
                        s.value().values().find_map(|v| match v {
                            Value::Reference(id) => Some(*id),
                            _ => None,
                        })
                    })
                    .or_else(|| {
                        model
                            .navigation_slot(current, kp::RELATIONSHIP_OWNING_RELATED_ELEMENT)
                            .and_then(|s| {
                                s.value().values().find_map(|v| match v {
                                    Value::Reference(id) => Some(*id),
                                    _ => None,
                                })
                            })
                    })
                else {
                    break;
                };
                current = owner;
            }
            println!(
                "{}",
                serde_json::json!({"result_subject":subject,"owners":owners,
                "value":actual.value,"completeness":format!("{:?}",actual.completeness),
                "diagnostics":format!("{:?}",actual.diagnostics),"publication_authority":false})
            );
        }
    }
    Ok(())
}
