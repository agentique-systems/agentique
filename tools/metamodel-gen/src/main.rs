use agq_metamodel_gen::{Result, pipeline};
use std::{fs, path::PathBuf};

fn run() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let mut check = false;
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut output = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--check" => check = true,
            "--root" => root = args.next().ok_or("--root needs a path")?.into(),
            "--output" => output = Some(PathBuf::from(args.next().ok_or("--output needs a path")?)),
            "--help" | "-h" => {
                println!(
                    "metamodel-gen [--check] [--root REPOSITORY] [--output FILE]\nImports hash-locked local XMI and cross-checks JSON. --check compares bytes without writing."
                );
                return Ok(());
            }
            _ => return Err(format!("unknown argument {arg}")),
        }
    }
    let bytes = pipeline::bytes(&root)?;
    let output = output.unwrap_or_else(|| root.join(pipeline::OUTPUT_PATH));
    // The generator can never be directed to overwrite an input or its lock.
    let resolved_root = root.canonicalize().map_err(|e| e.to_string())?;
    if output.exists() {
        let resolved = output.canonicalize().map_err(|e| e.to_string())?;
        if resolved.starts_with(resolved_root.join("standards/normative")) {
            return Err("output must not overwrite normative inputs or lock metadata".into());
        }
    }
    if check {
        let existing = fs::read(&output).map_err(|e| format!("{}: {e}", output.display()))?;
        if existing != bytes {
            return Err(format!("generated IR is stale: {}", output.display()));
        }
        println!(
            "Metamodel IR and normative JSON cross-check pass ({} bytes).",
            bytes.len()
        );
    } else {
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(&output, &bytes).map_err(|e| e.to_string())?;
        println!("Wrote {} ({} bytes).", output.display(), bytes.len());
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("metamodel-gen: {error}");
        std::process::exit(1);
    }
}
