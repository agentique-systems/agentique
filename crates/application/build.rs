use sha2::{Digest, Sha256};
use std::{
    env, fs,
    path::{Path, PathBuf},
};
fn collect(dir: &Path, files: &mut Vec<PathBuf>) {
    for e in fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            collect(&p, files)
        } else if matches!(p.extension().and_then(|s| s.to_str()), Some("rs" | "toml")) {
            files.push(p)
        }
    }
}
fn main() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    println!("cargo:rerun-if-changed={}", root.join("crates").display());
    println!("cargo:rerun-if-changed={}", root.join("adapters").display());
    let mut files = vec![
        root.join("Cargo.lock"),
        root.join("Cargo.toml"),
        root.join("rust-toolchain.toml"),
        root.join("standards/lock.json"),
    ];
    collect(&root.join("crates"), &mut files);
    collect(&root.join("adapters"), &mut files);
    files.sort();
    let mut hash = Sha256::new();
    hash.update(env::var("TARGET").unwrap());
    hash.update(env::var("PROFILE").unwrap());
    let compiler = std::process::Command::new(env::var("RUSTC").unwrap())
        .arg("--version")
        .output()
        .unwrap();
    assert!(compiler.status.success());
    hash.update(compiler.stdout);
    for p in files {
        println!("cargo:rerun-if-changed={}", p.display());
        hash.update(
            p.strip_prefix(&root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/"),
        );
        hash.update(fs::read(p).unwrap());
    }
    println!("cargo:rustc-env=AGQ_BUILD_ID={:x}", hash.finalize());
}
