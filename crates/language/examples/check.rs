//! Loads SysML files, validates them, prints the diagnostics and timings.
//!
//! ```text
//! cargo run -p agq-language --example check -- models/url-shortener
//! ```
//!
//! Exit code: 0 when the model is valid, 1 when there are diagnostics,
//! 2 when the files cannot be read.
use agq_language::{ElementKind, Source, Tree, library, parse, print, validate};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("usage: check <file-or-directory>...");
        return ExitCode::from(2);
    }
    let mut paths = Vec::new();
    for arg in &args {
        if let Err(error) = collect(Path::new(arg), &mut paths) {
            eprintln!("cannot read {arg}: {error}");
            return ExitCode::from(2);
        }
    }
    let mut sources = Vec::new();
    for path in &paths {
        match std::fs::read_to_string(path) {
            Ok(text) => sources.push(Source::new(path.display().to_string(), text)),
            Err(error) => {
                eprintln!("cannot read {}: {error}", path.display());
                return ExitCode::from(2);
            }
        }
    }
    library(); // the built-in library is parsed once per process

    let (mut tree, parse_time) = timed(|| parse(&sources));
    let (diagnostics, validate_time) = timed(|| validate(&tree));
    for d in &diagnostics {
        let at = d
            .location
            .map(|l| tree.describe_location(l))
            .unwrap_or_default();
        println!(
            "{at}: error[{}] {}: {}",
            d.code,
            tree.qualified_name(d.element),
            d.message
        );
    }
    let lines: usize = sources.iter().map(|s| s.text.lines().count()).sum();
    println!(
        "{} file(s), {lines} lines, {} elements: {}",
        sources.len(),
        tree.len(),
        match diagnostics.len() {
            0 => "valid".to_string(),
            n => format!("{n} problem(s)"),
        }
    );

    let (_, print_time) = timed(|| print(&tree));
    println!("parse    {}", ms(parse_time));
    println!("validate {}", ms(validate_time));
    println!("print    {}", ms(print_time));
    edit_timings(&mut tree);

    if diagnostics.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

/// Renames the first part usage, re-validates, renames it back, re-validates.
fn edit_timings(tree: &mut Tree) {
    let Some(part) = tree
        .walk()
        .into_iter()
        .find(|id| tree[*id].kind == ElementKind::Part && tree[*id].name.is_some())
    else {
        return;
    };
    let old = tree[part].name.clone().unwrap_or_default();
    let qualified = tree.qualified_name(part);
    let ((), rename_time) = timed(|| tree.get_mut(part).name = Some(format!("{old}Renamed")));
    let (after, validate_time) = timed(|| validate(tree));
    println!(
        "edit     rename part `{qualified}`: {} + re-validate {} -> {} problem(s)",
        ms(rename_time),
        ms(validate_time),
        after.len()
    );
    tree.get_mut(part).name = Some(old);
    let (after, validate_time) = timed(|| validate(tree));
    println!(
        "undo     rename back: re-validate {} -> {} problem(s)",
        ms(validate_time),
        after.len()
    );
}

fn collect(path: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if path.is_dir() {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(path)?
            .map(|e| e.map(|e| e.path()))
            .collect::<Result<_, _>>()?;
        entries.sort();
        for entry in entries {
            if entry.is_dir() || entry.extension().is_some_and(|e| e == "sysml") {
                collect(&entry, out)?;
            }
        }
        Ok(())
    } else if path.exists() {
        out.push(path.to_path_buf());
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "no such file or directory",
        ))
    }
}

fn timed<T>(f: impl FnOnce() -> T) -> (T, Duration) {
    let start = Instant::now();
    let value = f();
    (value, start.elapsed())
}

fn ms(duration: Duration) -> String {
    format!("{:.3} ms", duration.as_secs_f64() * 1000.0)
}
