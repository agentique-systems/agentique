use sha2::{Digest, Sha256};
use std::{
    env, fs,
    path::{Path, PathBuf},
};
fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            files(&p, out)
        } else if matches!(
            p.extension().and_then(|s| s.to_str()),
            Some("sysml" | "kerml")
        ) {
            out.push(p)
        }
    }
}
fn main() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("../..")
        .canonicalize()
        .unwrap();
    let mut paths = vec![];
    let lock: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("standards/lock.json")).unwrap()).unwrap();
    files(
        &root.join(
            lock["library_source_root"]
                .as_str()
                .unwrap_or("standards/libraries"),
        ),
        &mut paths,
    );
    paths.sort();
    let mut expected = std::collections::BTreeMap::new();
    for artifact in lock["artifacts"].as_array().unwrap() {
        if let Some(entries) = artifact["entries"].as_array() {
            let stem = Path::new(artifact["path"].as_str().unwrap())
                .file_stem()
                .unwrap()
                .to_str()
                .unwrap();
            for entry in entries {
                let path = entry["path"].as_str().unwrap();
                if path.ends_with(".sysml") || path.ends_with(".kerml") {
                    let directory = artifact["extract_root"]
                        .as_str()
                        .map(str::to_string)
                        .unwrap_or_else(|| format!("standards/libraries/{stem}"));
                    expected.insert(
                        format!("{directory}/{path}"),
                        entry["sha256"].as_str().unwrap().to_string(),
                    );
                }
            }
        }
    }
    assert_eq!(
        paths.len(),
        expected.len(),
        "Pinned library source set changed; run npm run standards:check"
    );
    let mut code = String::from("pub const LIBRARY_SOURCES: &[(&str,&str)] = &[\n");
    for p in paths {
        println!("cargo:rerun-if-changed={}", p.display());
        let name = p
            .strip_prefix(&root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let actual = format!("{:x}", Sha256::digest(fs::read(&p).unwrap()));
        assert_eq!(
            expected.get(&name),
            Some(&actual),
            "Official library changed: {name}"
        );
        code.push_str(&format!(
            "({name:?}, include_str!({:?})),\n",
            p.to_str().unwrap()
        ));
    }
    code.push_str("];\n");
    println!(
        "cargo:rerun-if-changed={}",
        root.join("standards/lock.json").display()
    );
    code.push_str(&format!(
        "pub const LIBRARY_LOCK: &str = include_str!({:?});\n",
        root.join("standards/lock.json").to_str().unwrap()
    ));
    fs::write(
        PathBuf::from(env::var("OUT_DIR").unwrap()).join("libraries.rs"),
        code,
    )
    .unwrap();
}
