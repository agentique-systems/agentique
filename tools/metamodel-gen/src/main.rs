use agq_metamodel_gen::{Result, baseline, canonical_json, descriptors, pipeline};
use std::{fs, path::PathBuf};

fn run() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let mut check = false;
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut output = None;
    let mut descriptor_output = None;
    let mut selected_baseline = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--check" => check = true,
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
                    "metamodel-gen [--check] [--baseline kerml-1.0|sysml-2.0] [--root REPOSITORY] [--output IR_FILE] [--descriptor-output DIRECTORY]\nImports reviewed offline profiles and cross-checks JSON. Default: both language IRs and KerML runtime outputs. --output alone selects KerML IR-only compatibility mode. --check compares bytes without writing."
                );
                return Ok(());
            }
            _ => return Err(format!("unknown argument {arg}")),
        }
    }
    let profile = selected_baseline.unwrap_or(baseline::KERML);
    let bundle = pipeline::generate_profile(&root, profile)?;
    let ir_only = output.is_some() && descriptor_output.is_none();
    let all = selected_baseline.is_none() && output.is_none();
    let mut outputs = vec![(
        output.unwrap_or_else(|| root.join(profile.output.expect("language output path"))),
        canonical_json(&bundle)?,
    )];
    if !ir_only && profile.descriptor_target == baseline::DescriptorTarget::KerMlRootCore {
        let directory = descriptor_output.unwrap_or_else(|| root.clone());
        outputs.extend(
            descriptors::artifacts(&bundle)?
                .into_iter()
                .map(|(path, bytes)| (directory.join(path), bytes)),
        );
    }
    if all {
        let sysml = pipeline::generate_profile(&root, baseline::SYSML)?;
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
