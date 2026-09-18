use agq_metamodel_gen::{Result, baseline, canonical_json, closure_audit, descriptors, pipeline};
use std::{fs, path::PathBuf};

fn run() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let mut check = false;
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut output = None;
    let mut descriptor_output = None;
    let mut selected_baseline = None;
    let mut require_runtime = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--check" => check = true,
            "--require-runtime" => require_runtime = true,
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
                    "metamodel-gen [--check] [--require-runtime] [--baseline kerml-1.0|sysml-2.0] [--root REPOSITORY] [--output IR_FILE] [--descriptor-output DIRECTORY]\nImports reviewed offline profiles and cross-checks JSON. Default: both language IRs, KerML runtime outputs and the SysML readiness audit. --output alone selects KerML IR-only compatibility mode. --check compares bytes without writing; it does not establish runtime readiness. --baseline sysml-2.0 --require-runtime fails while the structural audit is blocked."
                );
                return Ok(());
            }
            _ => return Err(format!("unknown argument {arg}")),
        }
    }
    let profile = selected_baseline.unwrap_or(baseline::KERML);
    let bundle = pipeline::generate_profile(&root, profile)?;
    if require_runtime && profile.descriptor_target == baseline::DescriptorTarget::SysMlStructural {
        let audit = closure_audit::report(&bundle)?;
        for diagnostic in audit["baseline_diagnostics"]
            .as_array()
            .into_iter()
            .flatten()
        {
            eprintln!(
                "Baseline diagnostic: {}#{} [{}; {}] {}",
                diagnostic["source"]["artifact_uri"]
                    .as_str()
                    .unwrap_or_default(),
                diagnostic["external_id"].as_str().unwrap_or_default(),
                diagnostic["source"]["sha256"].as_str().unwrap_or_default(),
                diagnostic["disposition"].as_str().unwrap_or_default(),
                diagnostic["governing_constraint"]
                    .as_str()
                    .unwrap_or_default()
            );
        }
        if audit["result"] != "representable" {
            return Err(format!(
                "SysML runtime closure blocked: {}. See {} for source-qualified evidence; no runtime output written.",
                audit["registration_error"],
                closure_audit::PATH
            ));
        }
    }
    let ir_only = output.is_some() && descriptor_output.is_none();
    let all = selected_baseline.is_none() && output.is_none();
    let mut outputs = vec![(
        output.unwrap_or_else(|| root.join(profile.output.expect("language output path"))),
        canonical_json(&bundle)?,
    )];
    if !ir_only && profile.descriptor_target == baseline::DescriptorTarget::KerMlRootCore {
        let directory = descriptor_output.as_ref().unwrap_or(&root);
        outputs.extend(
            descriptors::artifacts(&bundle)?
                .into_iter()
                .map(|(path, bytes)| (directory.join(path), bytes)),
        );
    }
    if !ir_only && profile.descriptor_target == baseline::DescriptorTarget::SysMlStructural {
        let directory = descriptor_output.as_ref().unwrap_or(&root);
        outputs.push((
            directory.join(closure_audit::PATH),
            closure_audit::bytes(&bundle)?,
        ));
    }
    if all {
        let sysml = pipeline::generate_profile(&root, baseline::SYSML)?;
        outputs.push((
            root.join(closure_audit::PATH),
            closure_audit::bytes(&sysml)?,
        ));
        outputs.push((
            root.join(baseline::SYSML.output.unwrap()),
            canonical_json(&sysml)?,
        ));
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
