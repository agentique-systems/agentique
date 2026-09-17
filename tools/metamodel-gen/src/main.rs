use agq_metamodel_gen::{Result, canonical_json, descriptors, pipeline};
use std::{fs, path::PathBuf};

fn run() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let mut check = false;
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut output = None;
    let mut descriptor_output = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--check" => check = true,
            "--root" => root = args.next().ok_or("--root needs a path")?.into(),
            "--output" => output = Some(PathBuf::from(args.next().ok_or("--output needs a path")?)),
            "--descriptor-output" => {
                descriptor_output = Some(PathBuf::from(
                    args.next().ok_or("--descriptor-output needs a directory")?,
                ))
            }
            "--help" | "-h" => {
                println!(
                    "metamodel-gen [--check] [--root REPOSITORY] [--output IR_FILE] [--descriptor-output DIRECTORY]\nImports hash-locked XMI and cross-checks JSON; emits IR, Root/Core Rust and golden manifest. --output alone selects IR-only compatibility mode. --check compares bytes without writing."
                );
                return Ok(());
            }
            _ => return Err(format!("unknown argument {arg}")),
        }
    }
    let bundle = pipeline::generate(&root)?;
    let ir_only = output.is_some() && descriptor_output.is_none();
    let mut outputs = vec![(
        output.unwrap_or_else(|| root.join(pipeline::OUTPUT_PATH)),
        canonical_json(&bundle)?,
    )];
    if !ir_only {
        let directory = descriptor_output.unwrap_or_else(|| root.clone());
        outputs.extend(
            descriptors::artifacts(&bundle)?
                .into_iter()
                .map(|(path, bytes)| (directory.join(path), bytes)),
        );
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
