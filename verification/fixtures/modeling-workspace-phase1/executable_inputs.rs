//! Source inputs shared by the parser preflight and held workspace tests.
#![allow(dead_code)]

pub const CONTRACTS: &str = include_str!("Contracts.kerml");
pub const REPOSITORY: &str = include_str!("Repository.sysml");
pub const WORKSPACE: &str = include_str!("Workspace.sysml");
pub const RECOVERED_REPOSITORY: &str = "package Storage { part def Repository {";
pub const WORKSPACE_PORT: &str = "        port bus;\n";
pub const WORKSPACE_SPECIALIZATION: &str =
    "    part def SpecializedWorkspace :> Workspace { part :>> repository; }\n";

pub fn contracts(index: usize) -> String {
    format!("package Contracts{index:03} {{ datatype RevisionValue; }}\n")
}

pub fn worker(index: usize) -> String {
    format!(
        "package Workbench{index:03} {{ part def Worker {{ attribute revision : Contracts{index:03}::RevisionValue; part engine; }} part worker : Worker; }}\n"
    )
}

pub fn with_port(source: &str) -> String {
    source.replacen("part engine;", "part engine; port bus;", 1)
}

pub fn with_specialization(source: &str) -> String {
    let closing_package = source.rfind('}').expect("complete package fixture");
    let mut next = source.to_owned();
    next.insert_str(
        closing_package,
        "part def SpecializedWorker :> Worker { part :>> engine; } ",
    );
    next
}
