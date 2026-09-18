use agq_metamodel_gen::{
    Result, baseline, canonical_json, closure_audit, descriptors, full_audit, pipeline,
};
use std::{fs, path::PathBuf};

fn run() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let mut check = false;
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut output = None;
    let mut descriptor_output = None;
    let mut selected_baseline = None;
    let mut require_runtime = false;
    let mut require_conformance = false;
    let mut audit_full = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--check" => check = true,
            "--require-runtime" => require_runtime = true,
            "--require-conformance" => require_conformance = true,
            "--audit-full" => audit_full = true,
            "--baseline" => {
                selected_baseline = Some(baseline::find(
                    &args.next().ok_or("--baseline needs an ID")?,
                )?)
            }
            "--root" => root = args.next().ok_or("--root needs a path")?.into(),
            "--output" => output = Some(PathBuf::from(args.next().ok_or("--output needs a path")?)),
            "--descriptor-output" => {
                descriptor_output = Some(PathBuf::from(
                    args.next().ok_or("--descriptor-output needs a directory")?,
                ))
            }
            "--help" | "-h" => {
                println!(
                    "metamodel-gen [--check] [--require-runtime] [--require-conformance] [--audit-full] [--baseline kerml-1.0|sysml-2.0] [--root REPOSITORY] [--output IR_FILE] [--descriptor-output DIRECTORY]\nImports reviewed offline profiles and cross-checks JSON. Default: both language IRs, complete KerML and SysML descriptors/views, retained Root/Core fixtures, current SysML closure audit, and full structural audits/manifests. --audit-full emits only complete audits/manifests. --output alone selects KerML IR-only compatibility mode. --check compares bytes without writing; it does not establish runtime readiness. --require-runtime requires the complete selected metamodel, including dependencies, to translate, register and support the required structural runtime algorithms; failures write nothing. --require-conformance additionally rejects authoring errors, including reviewed baseline anomalies. This is an implemented-rule audit, not full UML certification."
                );
                return Ok(());
            }
            _ => return Err(format!("unknown argument {arg}")),
        }
    }
    if audit_full && output.is_some() {
        return Err("--audit-full cannot be combined with --output".into());
    }
    let profile = selected_baseline.unwrap_or(baseline::KERML);
    if profile.output.is_none() {
        return Err("select a language baseline, not a primitive dependency".into());
    }
    let bundle = pipeline::generate_profile(&root, profile)?;
    if require_runtime || require_conformance {
        let audit = full_audit::report(&bundle)?;
        for finding in audit["findings"].as_array().into_iter().flatten() {
            eprintln!(
                "Full audit finding [{}]: {} {}",
                finding["category"],
                finding["code"],
                finding["source_qualified_id"].as_str().unwrap_or_else(|| {
                    finding["diagnostic"]["external_id"]
                        .as_str()
                        .unwrap_or_default()
                })
            );
        }
        for diagnostic in audit["conformance"].as_array().into_iter().flatten() {
            eprintln!(
                "Conformance {} {} {} ({})",
                diagnostic["severity"],
                diagnostic["rule"],
                diagnostic["source"]["external_id"],
                diagnostic["disposition"]
            );
        }
        if require_conformance
            && (audit["conformance_result"] != "conformant-to-implemented-rules"
                || audit["result"] != "representable")
        {
            return Err(format!(
                "strict conformance failed: {}",
                audit["conformance"]
            ));
        }
        if require_runtime && audit["result"] != "representable" {
            return Err(format!(
                "{} complete runtime blocked: translation={}, registration={}, runtime={}. See standards/generated/{}/full-audit.json; no runtime output written.",
                profile.specification,
                audit["translation_error"],
                audit["registration_error"],
                audit["runtime_errors"],
                profile.id
            ));
        }
    }
    let ir_only = output.is_some() && descriptor_output.is_none();
    let all = selected_baseline.is_none() && output.is_none();
    let mut outputs = if audit_full {
        vec![]
    } else {
        vec![(
            output.unwrap_or_else(|| root.join(profile.output.expect("language output path"))),
            canonical_json(&bundle)?,
        )]
    };
    if !ir_only {
        let directory = descriptor_output.as_ref().unwrap_or(&root);
        outputs.extend(
            full_audit::artifacts(&bundle, profile.id)?
                .into_iter()
                .map(|(path, bytes)| (directory.join(path), bytes)),
        );
    }
    if !audit_full
        && !ir_only
        && profile.descriptor_target == baseline::DescriptorTarget::KerMlComplete
    {
        let directory = descriptor_output.as_ref().unwrap_or(&root);
        outputs.extend(
            descriptors::artifacts(&bundle)?
                .into_iter()
                .map(|(path, bytes)| (directory.join(path), bytes)),
        );
    }
    if !audit_full
        && !ir_only
        && profile.descriptor_target == baseline::DescriptorTarget::SysMlComplete
    {
        let directory = descriptor_output.as_ref().unwrap_or(&root);
        outputs.push((
            directory.join(closure_audit::PATH),
            closure_audit::bytes(&bundle)?,
        ));
    }
    if all {
        let sysml = pipeline::generate_profile(&root, baseline::SYSML)?;
        let directory = descriptor_output.as_ref().unwrap_or(&root);
        outputs.extend(
            full_audit::artifacts(&sysml, baseline::SYSML.id)?
                .into_iter()
                .map(|(path, bytes)| (directory.join(path), bytes)),
        );
        if !audit_full {
            outputs.push((
                root.join(closure_audit::PATH),
                closure_audit::bytes(&sysml)?,
            ));
            outputs.push((
                root.join(baseline::SYSML.output.unwrap()),
                canonical_json(&sysml)?,
            ));
        }
    }
    if !audit_full && !ir_only {
        let directory = descriptor_output.as_ref().unwrap_or(&root);
        outputs.extend(
            descriptors::complete_artifacts(&bundle)?
                .into_iter()
                .map(|(p, b)| (directory.join(p), b)),
        );
        if all {
            let sysml = pipeline::generate_profile(&root, baseline::SYSML)?;
            outputs.extend(
                descriptors::complete_artifacts(&sysml)?
                    .into_iter()
                    .map(|(p, b)| (directory.join(p), b)),
            );
        }
    }
    // The generator can never be directed to overwrite an input or its lock.
    let resolved_root = root.canonicalize().map_err(|e| e.to_string())?;
    for (output, _) in &outputs {
        if output.exists() {
            let resolved = output.canonicalize().map_err(|e| e.to_string())?;
            if resolved.starts_with(resolved_root.join("standards/normative")) {
                return Err("output must not overwrite normative inputs or lock metadata".into());
            }
        }
    }
    // Validate every output before any write, and every comparison without writes.
    for (output, bytes) in outputs {
        if check {
            let existing = fs::read(&output).map_err(|e| format!("{}: {e}", output.display()))?;
            if existing != bytes {
                return Err(format!("generated output is stale: {}", output.display()));
            }
            println!("Current: {} ({} bytes).", output.display(), bytes.len());
        } else {
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            fs::write(&output, &bytes).map_err(|e| e.to_string())?;
            println!("Wrote {} ({} bytes).", output.display(), bytes.len());
        }
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("metamodel-gen: {error}");
        std::process::exit(1);
    }
}
